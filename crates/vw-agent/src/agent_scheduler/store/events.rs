//! 实现 SQLite 调度事件的追加和查询，保持事件顺序及领域字段的强类型映射。

use anyhow::{Context, Result};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde_json::Value;
use vw_api_types::common::TimestampMs;

use crate::agent_scheduler::domain::{AssigneeId, RunId, TaskId, TeamId};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::state_machine::AgentTaskStatus;

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const EVENT_COLUMNS: &str = "id, task_id, run_id, worker_id, team_id, kind, from_status, \
to_status, from_status_key, to_status_key, message, payload_json, created_at_ms";

#[derive(Debug, Clone, PartialEq)]
/// 追加审计事件所需的可选领域关联和状态变化信息。
pub struct AppendAgentEventRequest {
    pub task_id: Option<TaskId>,
    pub run_id: Option<RunId>,
    pub worker_id: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub kind: AgentEventKind,
    pub from_status: Option<AgentTaskStatus>,
    pub to_status: Option<AgentTaskStatus>,
    pub from_status_key: Option<String>,
    pub to_status_key: Option<String>,
    pub message: String,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq)]
/// 带单调游标的已持久化调度事件。
pub struct StoredAgentEvent {
    pub cursor: i64,
    pub task_id: Option<TaskId>,
    pub run_id: Option<RunId>,
    pub worker_id: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub kind: AgentEventKind,
    pub from_status: Option<AgentTaskStatus>,
    pub to_status: Option<AgentTaskStatus>,
    pub from_status_key: Option<String>,
    pub to_status_key: Option<String>,
    pub message: String,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

impl AgentSchedulerStore {
    /// 在当前作用域追加事件，并用强类型状态补齐缺省状态键。
    pub fn append_event(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        event: AppendAgentEventRequest,
    ) -> Result<StoredAgentEvent> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.append_event(context, event);
        }
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
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO scheduler_events (
                 platform_id, company_id, project_id, task_id, run_id, worker_id, team_id, kind,
                 from_status, to_status, from_status_key, to_status_key, message, payload_json,
                 created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                platform_id,
                company_id,
                project_id,
                event.task_id.as_ref().map(TaskId::as_str),
                event.run_id.as_ref().map(RunId::as_str),
                event.worker_id.as_ref().map(AssigneeId::as_str),
                event.team_id.as_ref().map(TeamId::as_str),
                event.kind.as_str(),
                event.from_status.map(AgentTaskStatus::as_str),
                event.to_status.map(AgentTaskStatus::as_str),
                from_status_key.as_deref(),
                to_status_key.as_deref(),
                event.message,
                payload_json,
                event.created_at_ms.0,
            ],
        )?;
        let cursor = tx.last_insert_rowid();
        let stored = load_event_with_conn(&tx, context, cursor)?
            .with_context(|| format!("event disappeared after append: {cursor}"))?;
        tx.commit()?;
        Ok(stored)
    }

    pub fn list_events_after(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        cursor: Option<i64>,
        limit: usize,
    ) -> Result<Vec<StoredAgentEvent>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit = i64::try_from(limit).context("event list limit out of range")?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_events_after(context, cursor, limit);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut stmt = conn.prepare(&format!(
            "SELECT {EVENT_COLUMNS}
             FROM scheduler_events
             WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id > ?4
             ORDER BY id
             LIMIT ?5"
        ))?;
        let rows = stmt.query_map(
            params![platform_id, company_id, project_id, cursor.unwrap_or(0), limit],
            map_event_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list agent events")
    }
}

fn load_event_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    cursor: i64,
) -> Result<Option<StoredAgentEvent>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {EVENT_COLUMNS} FROM scheduler_events
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, cursor], map_event_row)
        .optional()
        .with_context(|| format!("load agent event: {cursor}"))
}

fn map_event_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentEvent> {
    let task_id: Option<String> = row.get(1)?;
    let run_id: Option<String> = row.get(2)?;
    let worker_id: Option<String> = row.get(3)?;
    let team_id: Option<String> = row.get(4)?;
    let kind: String = row.get(5)?;
    let from_status: Option<String> = row.get(6)?;
    let to_status: Option<String> = row.get(7)?;
    let payload_json: String = row.get(11)?;

    Ok(StoredAgentEvent {
        cursor: row.get(0)?,
        task_id: task_id
            .map(TaskId::new)
            .transpose()
            .map_err(|error| conversion_error(1, error))?,
        run_id: run_id.map(RunId::new).transpose().map_err(|error| conversion_error(2, error))?,
        worker_id: worker_id
            .map(AssigneeId::new)
            .transpose()
            .map_err(|error| conversion_error(3, error))?,
        team_id: team_id
            .map(TeamId::new)
            .transpose()
            .map_err(|error| conversion_error(4, error))?,
        kind: AgentEventKind::parse(&kind).map_err(|error| conversion_error(5, error))?,
        from_status: from_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .map_err(|error| conversion_error(6, error))?,
        to_status: to_status
            .map(|status| AgentTaskStatus::parse(&status))
            .transpose()
            .map_err(|error| conversion_error(7, error))?,
        from_status_key: row.get(8)?,
        to_status_key: row.get(9)?,
        message: row.get(10)?,
        payload: serde_json::from_str(&payload_json)
            .map_err(|error| conversion_error(11, anyhow::Error::new(error)))?,
        created_at_ms: TimestampMs(row.get(12)?),
    })
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}
