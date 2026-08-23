//! 实现 SQLite 运行记录的认领、心跳和完成流程，确保任务状态与依赖均允许执行。

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{AgentRunStatus, RunId, TaskId, WorkerName};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const RUN_COLUMNS: &str = "id, task_id, worker_id, status, attempt, lease_acquired_at_ms, \
lease_expires_at_ms, started_at_ms, finished_at_ms, last_heartbeat_at_ms, error_message";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimTaskRunRequest {
    pub run_id: RunId,
    pub task_id: TaskId,
    pub worker_id: WorkerName,
    pub lease_acquired_at_ms: TimestampMs,
    pub lease_expires_at_ms: Option<TimestampMs>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimReadyTaskRunRequest {
    pub run_id: RunId,
    pub task_id: TaskId,
    pub worker_id: WorkerName,
    pub lease_acquired_at_ms: TimestampMs,
    pub lease_expires_at_ms: Option<TimestampMs>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAgentRun {
    pub id: RunId,
    pub task_id: TaskId,
    pub worker_id: Option<WorkerName>,
    pub status: AgentRunStatus,
    pub attempt: u32,
    pub lease_acquired_at_ms: Option<TimestampMs>,
    pub lease_expires_at_ms: Option<TimestampMs>,
    pub started_at_ms: Option<TimestampMs>,
    pub finished_at_ms: Option<TimestampMs>,
    pub last_heartbeat_at_ms: Option<TimestampMs>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunFinishStatus {
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

impl AgentSchedulerStore {
    pub fn load_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
    ) -> Result<Option<StoredAgentRun>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_run(context, run_id);
        }
        let conn = open_initialized_connection(self, context)?;
        load_run_with_conn(&conn, context, run_id)
    }

    pub fn load_latest_run_for_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Option<StoredAgentRun>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_latest_run_for_task(context, task_id);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {RUN_COLUMNS}
             FROM scheduler_runs
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND task_id = ?4
             ORDER BY attempt DESC,
                      COALESCE(finished_at_ms, last_heartbeat_at_ms, started_at_ms, lease_acquired_at_ms, 0) DESC,
                      id DESC
             LIMIT 1"
        ))?;
        stmt.query_row(params![platform_id, company_id, project_id, task_id.as_str()], map_run_row)
            .optional()
            .with_context(|| format!("load latest run for task: {}", task_id.as_str()))
    }

    pub fn claim_task_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: ClaimTaskRunRequest,
    ) -> Result<Option<StoredAgentRun>> {
        validate_claim_request(&request)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.claim_task_run(context, request);
        }

        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(task_status) = load_task_status(&tx, context, &request.task_id)? else {
            return Ok(None);
        };
        if !task_status_allows_run(task_status)
            || active_run_exists(&tx, context, &request.task_id)?
        {
            return Ok(None);
        }

        let attempt = next_attempt(&tx, context, &request.task_id)?;
        tx.execute(
            "INSERT INTO scheduler_runs (
                 platform_id, company_id, project_id, id, task_id, worker_id, status, attempt,
                 lease_acquired_at_ms, lease_expires_at_ms, started_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                platform_id,
                company_id,
                project_id,
                request.run_id.as_str(),
                request.task_id.as_str(),
                request.worker_id.as_str(),
                run_status_key(AgentRunStatus::Leased),
                i64::from(attempt),
                request.lease_acquired_at_ms.0,
                request.lease_expires_at_ms.map(|value| value.0),
                request.lease_acquired_at_ms.0,
            ],
        )?;

        let stored = load_run_with_conn(&tx, context, &request.run_id)?
            .with_context(|| format!("run disappeared after claim: {}", request.run_id.as_str()))?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn claim_ready_task_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: ClaimReadyTaskRunRequest,
    ) -> Result<Option<StoredAgentRun>> {
        validate_ready_claim_request(&request)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.claim_ready_task_run(context, request);
        }

        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some((kind, status)) = load_task_kind_and_status(&tx, context, &request.task_id)?
        else {
            return Ok(None);
        };
        if kind != AgentTaskKind::Finite
            || status != AgentTaskStatus::Queued
            || active_run_exists(&tx, context, &request.task_id)?
            || has_unsatisfied_dependencies(&tx, context, &request.task_id)?
        {
            return Ok(None);
        }

        let assigned = tx.execute(
            "UPDATE scheduler_tasks
             SET status = ?1, updated_at_ms = ?2
             WHERE platform_id = ?3 AND company_id = ?4 AND project_id = ?5 AND id = ?6
               AND kind = 'finite' AND status = 'queued'",
            params![
                AgentTaskStatus::Assigned.as_str(),
                request.lease_acquired_at_ms.0,
                platform_id,
                company_id,
                project_id,
                request.task_id.as_str(),
            ],
        )?;
        if assigned != 1 {
            return Ok(None);
        }

        let attempt = next_attempt(&tx, context, &request.task_id)?;
        tx.execute(
            "INSERT INTO scheduler_runs (
                 platform_id, company_id, project_id, id, task_id, worker_id, status, attempt,
                 lease_acquired_at_ms, lease_expires_at_ms, started_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                platform_id,
                company_id,
                project_id,
                request.run_id.as_str(),
                request.task_id.as_str(),
                request.worker_id.as_str(),
                run_status_key(AgentRunStatus::Leased),
                i64::from(attempt),
                request.lease_acquired_at_ms.0,
                request.lease_expires_at_ms.map(|value| value.0),
                request.lease_acquired_at_ms.0,
            ],
        )?;

        let stored = load_run_with_conn(&tx, context, &request.run_id)?.with_context(|| {
            format!("run disappeared after ready claim: {}", request.run_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn heartbeat_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
        heartbeat_at_ms: TimestampMs,
        lease_expires_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRun>> {
        if lease_expires_at_ms < heartbeat_at_ms {
            bail!("run lease expiry must not be before heartbeat time");
        }
        if let Some(store) = postgres_provider(self, context)? {
            return store.heartbeat_run(context, run_id, heartbeat_at_ms, lease_expires_at_ms);
        }

        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_runs
             SET status = ?1,
                 lease_expires_at_ms = ?2,
                 last_heartbeat_at_ms = ?3
             WHERE platform_id = ?4 AND company_id = ?5 AND project_id = ?6 AND id = ?7
               AND finished_at_ms IS NULL
               AND status IN ('leased', 'running')",
            params![
                run_status_key(AgentRunStatus::Running),
                lease_expires_at_ms.0,
                heartbeat_at_ms.0,
                platform_id,
                company_id,
                project_id,
                run_id.as_str(),
            ],
        )?;
        if updated == 0 {
            return Ok(None);
        }

        let stored = load_run_with_conn(&tx, context, run_id)?
            .with_context(|| format!("run disappeared after heartbeat: {}", run_id.as_str()))?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn finish_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
        status: AgentRunFinishStatus,
        finished_at_ms: TimestampMs,
        error_message: Option<String>,
    ) -> Result<Option<StoredAgentRun>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.finish_run(context, run_id, status, finished_at_ms, error_message);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_runs
             SET status = ?1,
                 finished_at_ms = ?2,
                 error_message = ?3
             WHERE platform_id = ?4 AND company_id = ?5 AND project_id = ?6 AND id = ?7
               AND finished_at_ms IS NULL
               AND status IN ('leased', 'running')",
            params![
                finish_status_key(status),
                finished_at_ms.0,
                error_message,
                platform_id,
                company_id,
                project_id,
                run_id.as_str(),
            ],
        )?;
        if updated == 0 {
            return Ok(None);
        }

        let stored = load_run_with_conn(&tx, context, run_id)?
            .with_context(|| format!("run disappeared after finish: {}", run_id.as_str()))?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn list_stale_runs(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
    ) -> Result<Vec<StoredAgentRun>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_stale_runs(context, now_ms);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {RUN_COLUMNS}
             FROM scheduler_runs
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
               AND finished_at_ms IS NULL
               AND status IN ('leased', 'running')
               AND lease_expires_at_ms IS NOT NULL
               AND lease_expires_at_ms <= ?4
             ORDER BY lease_expires_at_ms, id"
        ))?;
        let rows =
            stmt.query_map(params![platform_id, company_id, project_id, now_ms.0], map_run_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list stale agent runs")
    }
}

fn validate_claim_request(request: &ClaimTaskRunRequest) -> Result<()> {
    validate_lease_times(request.lease_acquired_at_ms, request.lease_expires_at_ms)
}

fn validate_ready_claim_request(request: &ClaimReadyTaskRunRequest) -> Result<()> {
    validate_lease_times(request.lease_acquired_at_ms, request.lease_expires_at_ms)
}

fn validate_lease_times(
    lease_acquired_at_ms: TimestampMs,
    lease_expires_at_ms: Option<TimestampMs>,
) -> Result<()> {
    if let Some(lease_expires_at_ms) = lease_expires_at_ms
        && lease_expires_at_ms < lease_acquired_at_ms
    {
        bail!("run lease expiry must not be before lease acquisition time");
    }
    Ok(())
}

fn load_task_kind_and_status(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<Option<(AgentTaskKind, AgentTaskStatus)>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let task = conn
        .query_row(
            "SELECT kind, status FROM scheduler_tasks
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4",
            params![platform_id, company_id, project_id, task_id.as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .with_context(|| format!("load task for ready run claim: {}", task_id.as_str()))?;
    task.map(|(kind, status)| {
        Ok((
            AgentTaskKind::parse(&kind).context("parse task kind for ready run claim")?,
            AgentTaskStatus::parse(&status).context("parse task status for ready run claim")?,
        ))
    })
    .transpose()
}

fn load_task_status(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<Option<AgentTaskStatus>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let status_key = conn
        .query_row(
            "SELECT status FROM scheduler_tasks
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4",
            params![platform_id, company_id, project_id, task_id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .with_context(|| format!("load task status for run claim: {}", task_id.as_str()))?;
    status_key
        .map(|status| AgentTaskStatus::parse(&status))
        .transpose()
        .with_context(|| format!("parse task status for run claim: {}", task_id.as_str()))
}

pub(super) fn task_status_allows_run(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned
            | AgentTaskStatus::Dispatched
            | AgentTaskStatus::Acknowledged
            | AgentTaskStatus::Active
            | AgentTaskStatus::Running
    )
}

fn active_run_exists(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<bool> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM scheduler_runs
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND task_id = ?4
           AND finished_at_ms IS NULL
           AND status IN ('leased', 'running')",
        params![platform_id, company_id, project_id, task_id.as_str()],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn has_unsatisfied_dependencies(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<bool> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM scheduler_task_dependencies AS dependency
         LEFT JOIN scheduler_tasks AS predecessor
           ON predecessor.platform_id = dependency.platform_id
          AND predecessor.company_id = dependency.company_id
          AND predecessor.project_id = dependency.project_id
          AND predecessor.id = dependency.predecessor_task_id
         WHERE dependency.platform_id = ?1
           AND dependency.company_id = ?2
           AND dependency.project_id = ?3
           AND dependency.successor_task_id = ?4
           AND (predecessor.id IS NULL OR predecessor.status <> 'completed')",
        params![platform_id, company_id, project_id, task_id.as_str()],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn next_attempt(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<u32> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let attempt: i64 = conn.query_row(
        "SELECT COALESCE(MAX(attempt), 0) + 1 FROM scheduler_runs
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND task_id = ?4",
        params![platform_id, company_id, project_id, task_id.as_str()],
        |row| row.get(0),
    )?;
    u32::try_from(attempt).with_context(|| {
        format!("run attempt out of range for task {}: {attempt}", task_id.as_str())
    })
}

fn load_run_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    run_id: &RunId,
) -> Result<Option<StoredAgentRun>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {RUN_COLUMNS} FROM scheduler_runs
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, run_id.as_str()], map_run_row)
        .optional()
        .with_context(|| format!("load agent run: {}", run_id.as_str()))
}

fn map_run_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentRun> {
    let status_key: String = row.get(3)?;
    let attempt: i64 = row.get(4)?;
    let worker_id: Option<String> = row.get(2)?;

    Ok(StoredAgentRun {
        id: parse_run_id(row.get(0)?)?,
        task_id: parse_task_id(row.get(1)?)?,
        worker_id: parse_optional_worker_name(worker_id)?,
        status: parse_run_status(&status_key).map_err(|error| conversion_error(3, error))?,
        attempt: i64_to_u32(attempt, 4)?,
        lease_acquired_at_ms: row.get::<_, Option<i64>>(5)?.map(TimestampMs),
        lease_expires_at_ms: row.get::<_, Option<i64>>(6)?.map(TimestampMs),
        started_at_ms: row.get::<_, Option<i64>>(7)?.map(TimestampMs),
        finished_at_ms: row.get::<_, Option<i64>>(8)?.map(TimestampMs),
        last_heartbeat_at_ms: row.get::<_, Option<i64>>(9)?.map(TimestampMs),
        error_message: row.get(10)?,
    })
}

fn parse_run_id(value: String) -> rusqlite::Result<RunId> {
    RunId::new(value).map_err(|error| conversion_error(0, error))
}

fn parse_task_id(value: String) -> rusqlite::Result<TaskId> {
    TaskId::new(value).map_err(|error| conversion_error(1, error))
}

fn parse_optional_worker_name(value: Option<String>) -> rusqlite::Result<Option<WorkerName>> {
    value.map(WorkerName::new).transpose().map_err(|error| conversion_error(2, error))
}

fn i64_to_u32(value: i64, column: usize) -> rusqlite::Result<u32> {
    u32::try_from(value)
        .map_err(|_| conversion_error(column, anyhow!("run attempt out of range: {value}")))
}

pub(super) fn run_status_key(status: AgentRunStatus) -> &'static str {
    match status {
        AgentRunStatus::Pending => "pending",
        AgentRunStatus::Leased => "leased",
        AgentRunStatus::Running => "running",
        AgentRunStatus::Succeeded => "succeeded",
        AgentRunStatus::Failed => "failed",
        AgentRunStatus::Cancelled => "cancelled",
        AgentRunStatus::TimedOut => "timed_out",
        AgentRunStatus::Stale => "stale",
    }
}

pub(super) fn parse_run_status(value: &str) -> Result<AgentRunStatus> {
    match value {
        "pending" => Ok(AgentRunStatus::Pending),
        "leased" => Ok(AgentRunStatus::Leased),
        "running" => Ok(AgentRunStatus::Running),
        "succeeded" => Ok(AgentRunStatus::Succeeded),
        "failed" => Ok(AgentRunStatus::Failed),
        "cancelled" => Ok(AgentRunStatus::Cancelled),
        "timed_out" => Ok(AgentRunStatus::TimedOut),
        "stale" => Ok(AgentRunStatus::Stale),
        other => bail!("unsupported agent run status: {other}"),
    }
}

pub(super) fn finish_status_key(status: AgentRunFinishStatus) -> &'static str {
    match status {
        AgentRunFinishStatus::Succeeded => run_status_key(AgentRunStatus::Succeeded),
        AgentRunFinishStatus::Failed => run_status_key(AgentRunStatus::Failed),
        AgentRunFinishStatus::TimedOut => run_status_key(AgentRunStatus::TimedOut),
        AgentRunFinishStatus::Cancelled => run_status_key(AgentRunStatus::Cancelled),
    }
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}
