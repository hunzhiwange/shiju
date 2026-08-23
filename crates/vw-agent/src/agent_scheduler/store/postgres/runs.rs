//! 实现 PostgreSQL 运行记录的原子认领、心跳与收尾，并检查任务依赖和活动租约。

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{AgentRunStatus, RunId, TaskId, WorkerName};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::runs::{
    AgentRunFinishStatus, ClaimReadyTaskRunRequest, ClaimTaskRunRequest, StoredAgentRun,
    finish_status_key, parse_run_status, run_status_key, task_status_allows_run,
};

const RUN_COLUMNS: &str = "id, task_id, worker_id, status, attempt, lease_acquired_at_ms, \
lease_expires_at_ms, started_at_ms, finished_at_ms, last_heartbeat_at_ms, error_message";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn load_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
    ) -> Result<Option<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            load_run_with_client(&mut *self.database.connect_postgres()?, &scope, run_id)
        })
    }

    pub(in crate::agent_scheduler::store) fn load_latest_run_for_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Option<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query_opt(
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM scheduler_runs
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                       AND task_id = $4
                     ORDER BY attempt DESC,
                              COALESCE(finished_at_ms, last_heartbeat_at_ms, started_at_ms,
                                       lease_acquired_at_ms, 0) DESC,
                              id DESC
                     LIMIT 1"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                    ],
                )
                .with_context(|| {
                    format!("load latest scoped PostgreSQL run for task: {}", task_id.as_str())
                })?
                .as_ref()
                .map(map_run_row)
                .transpose()
        })
    }

    pub(in crate::agent_scheduler::store) fn claim_task_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: ClaimTaskRunRequest,
    ) -> Result<Option<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction = connection.transaction().context("begin PostgreSQL run claim")?;
            let task_status = transaction
                .query_opt(
                    "SELECT status FROM scheduler_tasks
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4
                 FOR UPDATE SKIP LOCKED",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .with_context(|| {
                    format!("lock PostgreSQL task for run claim: {}", request.task_id.as_str())
                })?;
            let Some(task_status) = task_status else {
                return Ok(None);
            };
            let status_key: String =
                task_status.try_get(0).context("read PostgreSQL task status")?;
            let status = AgentTaskStatus::parse(&status_key)
                .context("parse PostgreSQL task claim status")?;
            if !task_status_allows_run(status) {
                return Ok(None);
            }

            let active_runs: i64 = transaction
                .query_one(
                    "SELECT COUNT(*) FROM scheduler_runs
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4
                   AND finished_at_ms IS NULL AND status IN ('leased', 'running')",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .context("inspect active PostgreSQL task runs")?
                .try_get(0)?;
            if active_runs > 0 {
                return Ok(None);
            }

            let attempt: i64 = transaction
                .query_one(
                    "SELECT COALESCE(MAX(attempt), 0) + 1 FROM scheduler_runs
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .context("select next PostgreSQL run attempt")?
                .try_get(0)?;
            u32::try_from(attempt).with_context(|| {
                format!("run attempt out of range for task {}: {attempt}", request.task_id.as_str())
            })?;

            transaction
                .execute(
                    "INSERT INTO scheduler_runs (
                     platform_id, company_id, project_id, id, task_id, worker_id, status, attempt,
                     lease_acquired_at_ms, lease_expires_at_ms, started_at_ms
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.run_id.as_str(),
                        &request.task_id.as_str(),
                        &request.worker_id.as_str(),
                        &run_status_key(AgentRunStatus::Leased),
                        &attempt,
                        &request.lease_acquired_at_ms.0,
                        &request.lease_expires_at_ms.map(|value| value.0),
                        &request.lease_acquired_at_ms.0,
                    ],
                )
                .with_context(|| {
                    format!("insert scoped PostgreSQL run claim: {}", request.run_id.as_str())
                })?;
            let stored = load_run_with_client(&mut transaction, &scope, &request.run_id)?
                .with_context(|| {
                    format!("PostgreSQL run disappeared after claim: {}", request.run_id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL run claim")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn claim_ready_task_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: ClaimReadyTaskRunRequest,
    ) -> Result<Option<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL ready task run claim")?;
            let task = transaction
                .query_opt(
                    "SELECT kind, status FROM scheduler_tasks
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4
                     FOR UPDATE",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .with_context(|| {
                    format!(
                        "lock PostgreSQL task for ready run claim: {}",
                        request.task_id.as_str()
                    )
                })?;
            let Some(task) = task else {
                return Ok(None);
            };
            let kind: String = task.try_get(0).context("read PostgreSQL ready claim task kind")?;
            let status: String =
                task.try_get(1).context("read PostgreSQL ready claim task status")?;
            if AgentTaskKind::parse(&kind).context("parse PostgreSQL ready claim task kind")?
                != AgentTaskKind::Finite
                || AgentTaskStatus::parse(&status)
                    .context("parse PostgreSQL ready claim task status")?
                    != AgentTaskStatus::Queued
                || active_run_exists(&mut transaction, &scope, &request.task_id)?
                || has_unsatisfied_dependencies(&mut transaction, &scope, &request.task_id)?
            {
                return Ok(None);
            }

            let assigned = transaction
                .execute(
                    "UPDATE scheduler_tasks
                     SET status = $1, updated_at_ms = $2
                     WHERE platform_id = $3 AND company_id = $4 AND project_id = $5 AND id = $6
                       AND kind = 'finite' AND status = 'queued'",
                    &[
                        &AgentTaskStatus::Assigned.as_str(),
                        &request.lease_acquired_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .context("assign PostgreSQL ready task before run claim")?;
            if assigned != 1 {
                return Ok(None);
            }

            let attempt: i64 = transaction
                .query_one(
                    "SELECT COALESCE(MAX(attempt), 0) + 1 FROM scheduler_runs
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .context("select next PostgreSQL ready claim run attempt")?
                .try_get(0)?;
            u32::try_from(attempt).with_context(|| {
                format!(
                    "run attempt out of range for ready task {}: {attempt}",
                    request.task_id.as_str()
                )
            })?;
            transaction
                .execute(
                    "INSERT INTO scheduler_runs (
                         platform_id, company_id, project_id, id, task_id, worker_id, status, attempt,
                         lease_acquired_at_ms, lease_expires_at_ms, started_at_ms
                     ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.run_id.as_str(),
                        &request.task_id.as_str(),
                        &request.worker_id.as_str(),
                        &run_status_key(AgentRunStatus::Leased),
                        &attempt,
                        &request.lease_acquired_at_ms.0,
                        &request.lease_expires_at_ms.map(|value| value.0),
                        &request.lease_acquired_at_ms.0,
                    ],
                )
                .with_context(|| {
                    format!("insert scoped PostgreSQL ready run claim: {}", request.run_id.as_str())
                })?;
            let stored = load_run_with_client(&mut transaction, &scope, &request.run_id)?
                .with_context(|| {
                    format!(
                        "PostgreSQL run disappeared after ready claim: {}",
                        request.run_id.as_str()
                    )
                })?;
            transaction.commit().context("commit PostgreSQL ready task run claim")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn heartbeat_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
        heartbeat_at_ms: TimestampMs,
        lease_expires_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL run heartbeat")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_runs
                 SET status = $1, lease_expires_at_ms = $2, last_heartbeat_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND project_id = $6 AND id = $7
                   AND finished_at_ms IS NULL AND status IN ('leased', 'running')",
                    &[
                        &run_status_key(AgentRunStatus::Running),
                        &lease_expires_at_ms.0,
                        &heartbeat_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &run_id.as_str(),
                    ],
                )
                .context("heartbeat scoped PostgreSQL run")?;
            match changed {
                0 => return Ok(None),
                1 => {}
                _ => anyhow::bail!(
                    "PostgreSQL run heartbeat affected {changed} rows: {}",
                    run_id.as_str()
                ),
            }
            let stored =
                load_run_with_client(&mut transaction, &scope, run_id)?.with_context(|| {
                    format!("PostgreSQL run disappeared after heartbeat: {}", run_id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL run heartbeat")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn finish_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
        status: AgentRunFinishStatus,
        finished_at_ms: TimestampMs,
        error_message: Option<String>,
    ) -> Result<Option<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL run finish")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_runs
                 SET status = $1, finished_at_ms = $2, error_message = $3
                 WHERE platform_id = $4 AND company_id = $5 AND project_id = $6 AND id = $7
                   AND finished_at_ms IS NULL AND status IN ('leased', 'running')",
                    &[
                        &finish_status_key(status),
                        &finished_at_ms.0,
                        &error_message,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &run_id.as_str(),
                    ],
                )
                .context("finish scoped PostgreSQL run")?;
            match changed {
                0 => return Ok(None),
                1 => {}
                _ => anyhow::bail!(
                    "PostgreSQL run finish affected {changed} rows: {}",
                    run_id.as_str()
                ),
            }
            let stored =
                load_run_with_client(&mut transaction, &scope, run_id)?.with_context(|| {
                    format!("PostgreSQL run disappeared after finish: {}", run_id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL run finish")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn list_stale_runs(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
    ) -> Result<Vec<StoredAgentRun>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {RUN_COLUMNS} FROM scheduler_runs
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                       AND finished_at_ms IS NULL AND status IN ('leased', 'running')
                       AND lease_expires_at_ms IS NOT NULL AND lease_expires_at_ms <= $4
                     ORDER BY lease_expires_at_ms, id"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &now_ms.0,
                    ],
                )
                .context("list scoped stale PostgreSQL runs")?
                .iter()
                .map(map_run_row)
                .collect()
        })
    }
}

fn active_run_exists(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_id: &TaskId,
) -> Result<bool> {
    let project = scope.project_scope();
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM scheduler_runs
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4
               AND finished_at_ms IS NULL AND status IN ('leased', 'running')",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &task_id.as_str(),
            ],
        )
        .context("inspect active PostgreSQL task runs for ready claim")?
        .try_get(0)?;
    Ok(count > 0)
}

fn has_unsatisfied_dependencies(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_id: &TaskId,
) -> Result<bool> {
    let project = scope.project_scope();
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*)
             FROM scheduler_task_dependencies AS dependency
             LEFT JOIN scheduler_tasks AS predecessor
               ON predecessor.platform_id = dependency.platform_id
              AND predecessor.company_id = dependency.company_id
              AND predecessor.project_id = dependency.project_id
              AND predecessor.id = dependency.predecessor_task_id
             WHERE dependency.platform_id = $1
               AND dependency.company_id = $2
               AND dependency.project_id = $3
               AND dependency.successor_task_id = $4
               AND (predecessor.id IS NULL OR predecessor.status <> 'completed')",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &task_id.as_str(),
            ],
        )
        .context("inspect PostgreSQL ready task dependencies")?
        .try_get(0)?;
    Ok(count > 0)
}

fn load_run_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    run_id: &RunId,
) -> Result<Option<StoredAgentRun>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {RUN_COLUMNS} FROM scheduler_runs
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &run_id.as_str(),
            ],
        )
        .with_context(|| format!("load scoped PostgreSQL agent run: {}", run_id.as_str()))?
        .as_ref()
        .map(map_run_row)
        .transpose()
}

fn map_run_row(row: &Row) -> Result<StoredAgentRun> {
    let run_id: String = row.try_get(0).context("read PostgreSQL run id")?;
    let task_id: String = row.try_get(1).context("read PostgreSQL run task id")?;
    let worker_id: Option<String> = row.try_get(2).context("read PostgreSQL run worker")?;
    let status_key: String = row.try_get(3).context("read PostgreSQL run status")?;
    let attempt: i64 = row.try_get(4).context("read PostgreSQL run attempt")?;
    Ok(StoredAgentRun {
        id: RunId::new(run_id).context("parse PostgreSQL run id")?,
        task_id: TaskId::new(task_id).context("parse PostgreSQL run task id")?,
        worker_id: worker_id
            .map(WorkerName::new)
            .transpose()
            .context("parse PostgreSQL run worker")?,
        status: parse_run_status(&status_key).context("parse PostgreSQL run status")?,
        attempt: u32::try_from(attempt)
            .with_context(|| format!("PostgreSQL run attempt out of range: {attempt}"))?,
        lease_acquired_at_ms: row.try_get::<_, Option<i64>>(5)?.map(TimestampMs),
        lease_expires_at_ms: row.try_get::<_, Option<i64>>(6)?.map(TimestampMs),
        started_at_ms: row.try_get::<_, Option<i64>>(7)?.map(TimestampMs),
        finished_at_ms: row.try_get::<_, Option<i64>>(8)?.map(TimestampMs),
        last_heartbeat_at_ms: row.try_get::<_, Option<i64>>(9)?.map(TimestampMs),
        error_message: row.try_get(10)?,
    })
}
