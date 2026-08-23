//! 实现 PostgreSQL 任务日志及运行工作区上下文的持久化与关联校验。
//! 日志序号在锁定任务日志集合后分配，确保并发追加仍保持单调且无重复。

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{RunId, TaskId};
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::logs::{
    AgentSchedulerWorkspaceContext, AppendAgentTaskLogRequest, StoredAgentRunWorkspaceContext,
    StoredAgentTaskLog,
};

const LOG_COLUMNS: &str = "task_id, run_id, seq_no, timestamp_ms, status_from, status_to, \
status_from_key, status_to_key, message";
const WORKSPACE_COLUMNS: &str = "task_id, run_id, slot_id, path, branch, recorded_at_ms";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn append_task_log(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: AppendAgentTaskLogRequest,
    ) -> Result<StoredAgentTaskLog> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL task log append")?;

            let task_exists = transaction
                .query_opt(
                    "SELECT id FROM scheduler_tasks
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4
                 FOR UPDATE",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .context("lock scoped PostgreSQL task for log append")?
                .is_some();
            if !task_exists {
                bail!("PostgreSQL task log task not found: {}", request.task_id.as_str());
            }
            ensure_run_matches_task(
                &mut transaction,
                &scope,
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
            let seq_no: i64 = transaction
                .query_one(
                    "SELECT COALESCE(MAX(seq_no), -1) + 1 FROM scheduler_task_logs
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                    ],
                )
                .context("select next scoped PostgreSQL task log sequence")?
                .try_get(0)
                .context("read PostgreSQL task log sequence")?;
            let inserted_seq_no: i64 = transaction
                .query_one(
                    "INSERT INTO scheduler_task_logs (
                     platform_id, company_id, project_id, task_id, run_id, seq_no,
                     timestamp_ms, status_from, status_to, status_from_key, status_to_key, message
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 RETURNING seq_no",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.task_id.as_str(),
                        &request.run_id.as_ref().map(RunId::as_str),
                        &seq_no,
                        &request.timestamp_ms.0,
                        &request.from_status.map(AgentTaskStatus::as_str),
                        &request.to_status.map(AgentTaskStatus::as_str),
                        &from_status_key.as_deref(),
                        &to_status_key.as_deref(),
                        &request.message,
                    ],
                )
                .context("insert scoped PostgreSQL task log")?
                .try_get(0)
                .context("read inserted PostgreSQL task log sequence")?;
            let stored =
                load_log_with_client(&mut transaction, &scope, &request.task_id, inserted_seq_no)?
                    .with_context(|| {
                        format!(
                            "PostgreSQL task log disappeared after append: {}#{inserted_seq_no}",
                            request.task_id.as_str()
                        )
                    })?;
            transaction.commit().context("commit PostgreSQL task log append")?;
            Ok(stored)
        })
    }

    pub(in crate::agent_scheduler::store) fn list_task_logs(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Vec<StoredAgentTaskLog>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {LOG_COLUMNS} FROM scheduler_task_logs
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                       AND task_id = $4
                     ORDER BY seq_no"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                    ],
                )
                .context("list scoped PostgreSQL task logs")?
                .iter()
                .map(map_log_row)
                .collect()
        })
    }

    pub(in crate::agent_scheduler::store) fn record_run_workspace_context(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        run_id: &RunId,
        workspace: AgentSchedulerWorkspaceContext,
        recorded_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRunWorkspaceContext>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL workspace record")?;
            let Some(run_task_id) = load_run_task_id(&mut transaction, &scope, run_id, true)?
            else {
                return Ok(None);
            };
            if run_task_id.as_str() != task_id.as_str() {
                bail!("run {} does not belong to task {}", run_id.as_str(), task_id.as_str());
            }

            transaction
                .query_one(
                    "INSERT INTO scheduler_run_workspaces (
                     platform_id, company_id, project_id, task_id, run_id, slot_id, path, branch,
                     recorded_at_ms
                 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (platform_id, company_id, project_id, run_id) DO UPDATE SET
                     task_id = EXCLUDED.task_id,
                     slot_id = EXCLUDED.slot_id,
                     path = EXCLUDED.path,
                     branch = EXCLUDED.branch,
                     recorded_at_ms = EXCLUDED.recorded_at_ms
                 RETURNING run_id",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                        &run_id.as_str(),
                        &workspace.slot_id,
                        &workspace.path,
                        &workspace.branch,
                        &recorded_at_ms.0,
                    ],
                )
                .context("record scoped PostgreSQL run workspace")?;
            let stored = load_workspace_with_client(&mut transaction, &scope, run_id)?
                .with_context(|| {
                    format!("PostgreSQL workspace disappeared after record: {}", run_id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL workspace record")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn load_run_workspace_context(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        run_id: &RunId,
    ) -> Result<Option<StoredAgentRunWorkspaceContext>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            load_workspace_with_client(&mut *self.database.connect_postgres()?, &scope, run_id)
        })
    }
}

fn ensure_run_matches_task(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    run_id: Option<&RunId>,
    task_id: &TaskId,
    record_kind: &str,
) -> Result<()> {
    let Some(run_id) = run_id else {
        return Ok(());
    };
    let Some(run_task_id) = load_run_task_id(client, scope, run_id, false)? else {
        bail!("{record_kind} run not found: {}", run_id.as_str());
    };
    if run_task_id.as_str() != task_id.as_str() {
        bail!("{record_kind} run {} does not belong to task {}", run_id.as_str(), task_id.as_str());
    }
    Ok(())
}

fn load_run_task_id(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    run_id: &RunId,
    lock: bool,
) -> Result<Option<TaskId>> {
    let project = scope.project_scope();
    let lock_clause = if lock { " FOR UPDATE" } else { "" };
    client
        .query_opt(
            &format!(
                "SELECT task_id FROM scheduler_runs
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4{lock_clause}"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &run_id.as_str(),
            ],
        )
        .context("load scoped PostgreSQL run task")?
        .map(|row| {
            let task_id: String = row.try_get(0).context("read PostgreSQL run task id")?;
            TaskId::new(task_id).context("parse PostgreSQL run task id")
        })
        .transpose()
}

fn load_log_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_id: &TaskId,
    seq_no: i64,
) -> Result<Option<StoredAgentTaskLog>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {LOG_COLUMNS} FROM scheduler_task_logs
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND task_id = $4 AND seq_no = $5"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &task_id.as_str(),
                &seq_no,
            ],
        )
        .context("load scoped PostgreSQL task log")?
        .as_ref()
        .map(map_log_row)
        .transpose()
}

fn load_workspace_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    run_id: &RunId,
) -> Result<Option<StoredAgentRunWorkspaceContext>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {WORKSPACE_COLUMNS} FROM scheduler_run_workspaces
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND run_id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &run_id.as_str(),
            ],
        )
        .context("load scoped PostgreSQL run workspace")?
        .as_ref()
        .map(map_workspace_row)
        .transpose()
}

fn map_log_row(row: &Row) -> Result<StoredAgentTaskLog> {
    let task_id: String = row.try_get(0).context("read PostgreSQL task log task id")?;
    let run_id: Option<String> = row.try_get(1).context("read PostgreSQL task log run id")?;
    let from_status: Option<String> =
        row.try_get(4).context("read PostgreSQL task log source status")?;
    let to_status: Option<String> =
        row.try_get(5).context("read PostgreSQL task log target status")?;
    Ok(StoredAgentTaskLog {
        task_id: TaskId::new(task_id).context("parse PostgreSQL task log task id")?,
        run_id: run_id.map(RunId::new).transpose().context("parse PostgreSQL task log run id")?,
        seq_no: row.try_get(2).context("read PostgreSQL task log sequence")?,
        timestamp_ms: TimestampMs(row.try_get(3).context("read PostgreSQL task log timestamp")?),
        from_status: from_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .context("parse PostgreSQL task log source status")?,
        to_status: to_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .context("parse PostgreSQL task log target status")?,
        from_status_key: row.try_get(6).context("read PostgreSQL task log source status key")?,
        to_status_key: row.try_get(7).context("read PostgreSQL task log target status key")?,
        message: row.try_get(8).context("read PostgreSQL task log message")?,
    })
}

fn map_workspace_row(row: &Row) -> Result<StoredAgentRunWorkspaceContext> {
    let task_id: String = row.try_get(0).context("read PostgreSQL workspace task id")?;
    let run_id: String = row.try_get(1).context("read PostgreSQL workspace run id")?;
    Ok(StoredAgentRunWorkspaceContext {
        task_id: TaskId::new(task_id).context("parse PostgreSQL workspace task id")?,
        run_id: RunId::new(run_id).context("parse PostgreSQL workspace run id")?,
        workspace: AgentSchedulerWorkspaceContext {
            slot_id: row.try_get(2).context("read PostgreSQL workspace slot")?,
            path: row.try_get(3).context("read PostgreSQL workspace path")?,
            branch: row.try_get(4).context("read PostgreSQL workspace branch")?,
        },
        recorded_at_ms: TimestampMs(row.try_get(5).context("read PostgreSQL workspace timestamp")?),
    })
}

#[cfg(test)]
#[path = "logs_tests.rs"]
mod tests;
