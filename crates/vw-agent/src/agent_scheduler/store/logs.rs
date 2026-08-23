//! 实现 SQLite 任务日志和运行工作区上下文的追加、读取与关联一致性检查。

use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::state_machine::AgentTaskStatus;

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const LOG_COLUMNS: &str = "task_id, run_id, seq_no, timestamp_ms, status_from, status_to, \
status_from_key, status_to_key, message";
const WORKSPACE_COLUMNS: &str = "task_id, run_id, slot_id, path, branch, recorded_at_ms";

#[derive(Debug, Clone, PartialEq, Eq)]
/// 追加任务日志所需的运行关联和可选状态变化。
pub struct AppendAgentTaskLogRequest {
    pub task_id: TaskId,
    pub run_id: Option<RunId>,
    pub timestamp_ms: TimestampMs,
    pub from_status: Option<AgentTaskStatus>,
    pub to_status: Option<AgentTaskStatus>,
    pub from_status_key: Option<String>,
    pub to_status_key: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 带任务内单调序号的已持久化日志行。
pub struct StoredAgentTaskLog {
    pub task_id: TaskId,
    pub run_id: Option<RunId>,
    pub seq_no: i64,
    pub timestamp_ms: TimestampMs,
    pub from_status: Option<AgentTaskStatus>,
    pub to_status: Option<AgentTaskStatus>,
    pub from_status_key: Option<String>,
    pub to_status_key: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 执行器分配给运行的工作树槽位、路径和分支。
pub struct AgentSchedulerWorkspaceContext {
    pub slot_id: String,
    pub path: String,
    pub branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 与任务和运行绑定的已持久化工作区上下文。
pub struct StoredAgentRunWorkspaceContext {
    pub task_id: TaskId,
    pub run_id: RunId,
    pub workspace: AgentSchedulerWorkspaceContext,
    pub recorded_at_ms: TimestampMs,
}

impl AgentSchedulerStore {
    /// 校验运行归属后追加日志，并分配任务作用域内的下一个序号。
    pub fn append_task_log(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: AppendAgentTaskLogRequest,
    ) -> Result<StoredAgentTaskLog> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.append_task_log(context, request);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_run_matches_task(
            &tx,
            context,
            request.run_id.as_ref(),
            &request.task_id,
            "task log",
        )?;
        let from_status_key = request
            .from_status_key
            .clone()
            .or_else(|| request.from_status.map(AgentTaskStatus::as_str).map(str::to_string));
        let to_status_key = request
            .to_status_key
            .clone()
            .or_else(|| request.to_status.map(AgentTaskStatus::as_str).map(str::to_string));
        let seq_no = tx.query_row(
            "SELECT COALESCE(MAX(seq_no), -1) + 1 FROM scheduler_task_logs
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND task_id = ?4",
            params![platform_id, company_id, project_id, request.task_id.as_str()],
            |row| row.get::<_, i64>(0),
        )?;
        tx.execute(
            "INSERT INTO scheduler_task_logs (
                platform_id, company_id, project_id, task_id, run_id, seq_no,
                timestamp_ms, status_from, status_to,
                status_from_key, status_to_key, message
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                platform_id,
                company_id,
                project_id,
                request.task_id.as_str(),
                request.run_id.as_ref().map(RunId::as_str),
                seq_no,
                request.timestamp_ms.0,
                request.from_status.map(AgentTaskStatus::as_str),
                request.to_status.map(AgentTaskStatus::as_str),
                from_status_key.as_deref(),
                to_status_key.as_deref(),
                request.message,
            ],
        )?;
        let stored =
            load_log_with_conn(&tx, context, &request.task_id, seq_no)?.with_context(|| {
                format!("task log disappeared after append: {}#{seq_no}", request.task_id.as_str())
            })?;
        tx.commit()?;
        Ok(stored)
    }

    pub fn list_task_logs(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Vec<StoredAgentTaskLog>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_task_logs(context, task_id);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {LOG_COLUMNS}
             FROM scheduler_task_logs
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND task_id = ?4
             ORDER BY seq_no"
        ))?;
        let rows = stmt.query_map(
            params![platform_id, company_id, project_id, task_id.as_str()],
            map_log_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list agent scheduler task logs")
    }

    pub fn record_run_workspace_context(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        workspace: AgentSchedulerWorkspaceContext,
        recorded_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRunWorkspaceContext>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.record_run_workspace_context(
                context,
                task_id,
                run_id,
                workspace,
                recorded_at_ms,
            );
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(run_task_id) = load_run_task_id(&tx, context, run_id)? else {
            return Ok(None);
        };
        if run_task_id.as_str() != task_id.as_str() {
            bail!("run {} does not belong to task {}", run_id.as_str(), task_id.as_str());
        }

        tx.execute(
            "INSERT INTO scheduler_run_workspaces (
                platform_id, company_id, project_id, task_id, run_id, slot_id, path, branch,
                recorded_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(platform_id, company_id, project_id, run_id) DO UPDATE SET
                task_id = excluded.task_id,
                slot_id = excluded.slot_id,
                path = excluded.path,
                branch = excluded.branch,
                recorded_at_ms = excluded.recorded_at_ms",
            params![
                platform_id,
                company_id,
                project_id,
                task_id.as_str(),
                run_id.as_str(),
                workspace.slot_id,
                workspace.path,
                workspace.branch,
                recorded_at_ms.0,
            ],
        )?;
        let stored = load_workspace_with_conn(&tx, context, run_id)?.with_context(|| {
            format!("workspace context disappeared after record: {}", run_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn load_run_workspace_context(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
    ) -> Result<Option<StoredAgentRunWorkspaceContext>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_run_workspace_context(context, run_id);
        }
        let conn = open_initialized_connection(self, context)?;
        load_workspace_with_conn(&conn, context, run_id)
    }
}

fn ensure_run_matches_task(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    run_id: Option<&RunId>,
    task_id: &TaskId,
    record_kind: &str,
) -> Result<()> {
    let Some(run_id) = run_id else {
        return Ok(());
    };
    let Some(run_task_id) = load_run_task_id(conn, context, run_id)? else {
        bail!("{record_kind} run not found: {}", run_id.as_str());
    };
    if run_task_id.as_str() != task_id.as_str() {
        bail!("{record_kind} run {} does not belong to task {}", run_id.as_str(), task_id.as_str());
    }
    Ok(())
}

fn load_run_task_id(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    run_id: &RunId,
) -> Result<Option<TaskId>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    conn.query_row(
        "SELECT task_id FROM scheduler_runs
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4",
        params![platform_id, company_id, project_id, run_id.as_str()],
        |row| row.get::<_, String>(0),
    )
    .optional()?
    .map(TaskId::new)
    .transpose()
}

fn load_log_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
    seq_no: i64,
) -> Result<Option<StoredAgentTaskLog>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {LOG_COLUMNS} FROM scheduler_task_logs
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
           AND task_id = ?4 AND seq_no = ?5"
    ))?;
    stmt.query_row(
        params![platform_id, company_id, project_id, task_id.as_str(), seq_no],
        map_log_row,
    )
    .optional()
    .with_context(|| format!("load agent scheduler task log: {}#{seq_no}", task_id.as_str()))
}

fn load_workspace_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    run_id: &RunId,
) -> Result<Option<StoredAgentRunWorkspaceContext>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {WORKSPACE_COLUMNS} FROM scheduler_run_workspaces
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND run_id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, run_id.as_str()], map_workspace_row)
        .optional()
        .with_context(|| format!("load agent scheduler run workspace: {}", run_id.as_str()))
}

pub(crate) fn map_log_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentTaskLog> {
    let task_id: String = row.get(0)?;
    let run_id: Option<String> = row.get(1)?;
    let from_status: Option<String> = row.get(4)?;
    let to_status: Option<String> = row.get(5)?;

    Ok(StoredAgentTaskLog {
        task_id: TaskId::new(task_id).map_err(|error| conversion_error(0, error))?,
        run_id: run_id.map(RunId::new).transpose().map_err(|error| conversion_error(1, error))?,
        seq_no: row.get(2)?,
        timestamp_ms: TimestampMs(row.get(3)?),
        from_status: from_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .map_err(|error| conversion_error(4, error))?,
        to_status: to_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .map_err(|error| conversion_error(5, error))?,
        from_status_key: row.get(6)?,
        to_status_key: row.get(7)?,
        message: row.get(8)?,
    })
}

pub(crate) fn map_workspace_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentRunWorkspaceContext> {
    Ok(StoredAgentRunWorkspaceContext {
        task_id: TaskId::new(row.get::<_, String>(0)?)
            .map_err(|error| conversion_error(0, error))?,
        run_id: RunId::new(row.get::<_, String>(1)?).map_err(|error| conversion_error(1, error))?,
        workspace: AgentSchedulerWorkspaceContext {
            slot_id: row.get(2)?,
            path: row.get(3)?,
            branch: row.get(4)?,
        },
        recorded_at_ms: TimestampMs(row.get(5)?),
    })
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}
