//! 实现 PostgreSQL 调度事件追加与查询，并完成数据库行到领域事件的校验映射。
//! 事件游标由数据库生成，读取始终限定调度作用域并保持游标顺序。

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::domain::{RunId, TaskId, TeamId, WorkerName};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::events::{AppendAgentEventRequest, StoredAgentEvent};

const EVENT_COLUMNS: &str = "id, task_id, run_id, worker_id, team_id, kind, from_status, \
to_status, from_status_key, to_status_key, message, payload_json, created_at_ms";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn append_event(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        event: AppendAgentEventRequest,
    ) -> Result<StoredAgentEvent> {
        crate::global_database::run_sync_database(|| {
            let payload_json =
                serde_json::to_string(&event.payload).context("serialize event payload")?;
            let from_status_key = event
                .from_status_key
                .clone()
                .or_else(|| event.from_status.map(AgentTaskStatus::as_str).map(str::to_string));
            let to_status_key = event
                .to_status_key
                .clone()
                .or_else(|| event.to_status.map(AgentTaskStatus::as_str).map(str::to_string));
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL scheduler event append")?;
            let cursor: i64 = transaction
                .query_one(
                    "INSERT INTO scheduler_events (
                     platform_id, company_id, project_id, task_id, run_id, worker_id, team_id, kind,
                     from_status, to_status, from_status_key, to_status_key, message, payload_json,
                     created_at_ms
                 ) VALUES (
                     $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
                 )
                 RETURNING id",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &event.task_id.as_ref().map(TaskId::as_str),
                        &event.run_id.as_ref().map(RunId::as_str),
                        &event.worker_id.as_ref().map(WorkerName::as_str),
                        &event.team_id.as_ref().map(TeamId::as_str),
                        &event.kind.as_str(),
                        &event.from_status.map(AgentTaskStatus::as_str),
                        &event.to_status.map(AgentTaskStatus::as_str),
                        &from_status_key.as_deref(),
                        &to_status_key.as_deref(),
                        &event.message,
                        &payload_json,
                        &event.created_at_ms.0,
                    ],
                )
                .context("insert scoped PostgreSQL scheduler event")?
                .try_get(0)
                .context("read inserted PostgreSQL scheduler event cursor")?;
            let stored =
                load_event_with_client(&mut transaction, &scope, cursor)?.with_context(|| {
                    format!("PostgreSQL scheduler event disappeared after append: {cursor}")
                })?;
            transaction.commit().context("commit PostgreSQL scheduler event append")?;
            Ok(stored)
        })
    }

    pub(in crate::agent_scheduler::store) fn list_events_after(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        cursor: Option<i64>,
        limit: i64,
    ) -> Result<Vec<StoredAgentEvent>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            self.database
                .connect_postgres()?
                .query(
                    &format!(
                        "SELECT {EVENT_COLUMNS} FROM scheduler_events
                     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id > $4
                     ORDER BY id
                     LIMIT $5"
                    ),
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &cursor.unwrap_or(0),
                        &limit,
                    ],
                )
                .context("list scoped PostgreSQL scheduler events")?
                .iter()
                .map(map_event_row)
                .collect()
        })
    }
}

fn load_event_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    cursor: i64,
) -> Result<Option<StoredAgentEvent>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {EVENT_COLUMNS} FROM scheduler_events
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &cursor,
            ],
        )
        .context("load scoped PostgreSQL scheduler event")?
        .as_ref()
        .map(map_event_row)
        .transpose()
}

fn map_event_row(row: &Row) -> Result<StoredAgentEvent> {
    let task_id: Option<String> = row.try_get(1).context("read PostgreSQL event task id")?;
    let run_id: Option<String> = row.try_get(2).context("read PostgreSQL event run id")?;
    let worker_id: Option<String> = row.try_get(3).context("read PostgreSQL event worker id")?;
    let team_id: Option<String> = row.try_get(4).context("read PostgreSQL event team id")?;
    let kind: String = row.try_get(5).context("read PostgreSQL event kind")?;
    let from_status: Option<String> =
        row.try_get(6).context("read PostgreSQL event source status")?;
    let to_status: Option<String> =
        row.try_get(7).context("read PostgreSQL event target status")?;
    let payload_json: String = row.try_get(11).context("read PostgreSQL event payload")?;
    Ok(StoredAgentEvent {
        cursor: row.try_get(0).context("read PostgreSQL event cursor")?,
        task_id: task_id.map(TaskId::new).transpose().context("parse PostgreSQL event task id")?,
        run_id: run_id.map(RunId::new).transpose().context("parse PostgreSQL event run id")?,
        worker_id: worker_id
            .map(WorkerName::new)
            .transpose()
            .context("parse PostgreSQL event worker id")?,
        team_id: team_id.map(TeamId::new).transpose().context("parse PostgreSQL event team id")?,
        kind: AgentEventKind::parse(&kind).context("parse PostgreSQL event kind")?,
        from_status: from_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .context("parse PostgreSQL event source status")?,
        to_status: to_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .context("parse PostgreSQL event target status")?,
        from_status_key: row.try_get(8).context("read PostgreSQL event source status key")?,
        to_status_key: row.try_get(9).context("read PostgreSQL event target status key")?,
        message: row.try_get(10).context("read PostgreSQL event message")?,
        payload: serde_json::from_str(&payload_json)
            .context("parse PostgreSQL scheduler event payload")?,
        created_at_ms: TimestampMs(row.try_get(12).context("read PostgreSQL event timestamp")?),
    })
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
