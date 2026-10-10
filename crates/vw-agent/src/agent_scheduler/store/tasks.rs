//! 实现 SQLite 任务与看板数据的创建、读取、筛选和受状态机约束的更新。

use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto, TaskPoolWorkflowTransitionDto,
};

use crate::agent_scheduler::domain::{TaskId, TeamId, AssigneeId};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus, ensure_transition};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

const TASK_COLUMNS: &str = "id, kind, status, title, spec_uri, meta_uri, assigned_to, team_id, \
room_id, project_room_id, delegated_to_team, schedule, timezone, next_scheduled_at, \
last_executed_at, dag_json, metadata_json, created_at_ms, updated_at_ms";
const TASK_BOARD_WORKFLOW_META_KEY: &str = "task_board.workflow";

#[derive(Debug, Clone, PartialEq)]
/// 创建任务所需的完整领域字段；写入前会校验种类与调度字段的一致性。
pub struct NewAgentTask {
    pub id: TaskId,
    pub title: String,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
    pub spec_uri: String,
    pub meta_uri: String,
    pub assigned_to: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub room_id: Option<String>,
    pub project_room_id: Option<String>,
    pub delegated_to_team: Option<TeamId>,
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    pub next_scheduled_at: Option<String>,
    pub last_executed_at: Option<String>,
    pub dag: Vec<AgentTaskDagNode>,
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq)]
/// 从存储读取的任务快照，包括持久化时间和看板元数据。
pub struct StoredAgentTask {
    pub id: TaskId,
    pub title: String,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
    pub spec_uri: String,
    pub meta_uri: String,
    pub assigned_to: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub room_id: Option<String>,
    pub project_room_id: Option<String>,
    pub delegated_to_team: Option<TeamId>,
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    pub next_scheduled_at: Option<String>,
    pub last_executed_at: Option<String>,
    pub dag: Vec<AgentTaskDagNode>,
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 存储层使用的团队任务依赖图节点。
pub struct AgentTaskDagNode {
    pub task_id: TaskId,
    pub title: String,
    pub depends_on: Vec<TaskId>,
    pub assigned_to: Option<AssigneeId>,
    pub status: AgentTaskStatus,
}

#[derive(Debug, Clone, Default)]
/// 任务列表查询条件；所有已设置条件按“且”组合。
pub struct AgentTaskFilter {
    pub task_id: Option<TaskId>,
    pub status: Option<AgentTaskStatus>,
    pub assigned_to: Option<AssigneeId>,
}

#[derive(Debug, Clone, PartialEq)]
/// 看板任务一次更新中允许变更的状态、标题和元数据。
pub struct AgentTaskBoardUpdate {
    pub title: Option<String>,
    pub status: Option<AgentTaskStatus>,
    pub metadata: Value,
    pub updated_at_ms: TimestampMs,
}

impl AgentTaskFilter {
    /// 创建仅匹配指定任务的筛选器。
    pub fn for_task(task_id: TaskId) -> Self {
        Self { task_id: Some(task_id), ..Self::default() }
    }

    /// 创建仅匹配指定状态的筛选器。
    pub fn for_status(status: AgentTaskStatus) -> Self {
        Self { status: Some(status), ..Self::default() }
    }

    /// 创建仅匹配指定工作节点归属的筛选器。
    pub fn for_worker(assigned_to: AssigneeId) -> Self {
        Self { assigned_to: Some(assigned_to), ..Self::default() }
    }
}

impl AgentSchedulerStore {
    /// 在作用域内创建唯一任务，并返回规范化后的持久化快照。
    pub fn create_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task: NewAgentTask,
    ) -> Result<StoredAgentTask> {
        validate_new_task(&task)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.create_task(context, task);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let dag_json = serialize_dag(&task.dag)?;
        let metadata_json =
            serde_json::to_string(&task.metadata).context("serialize task metadata")?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO scheduler_tasks (
                 platform_id, company_id, project_id, id, kind, status, title, spec_uri, meta_uri,
                 assigned_to, team_id, room_id, project_room_id, delegated_to_team, schedule,
                 timezone, next_scheduled_at, last_executed_at, dag_json, metadata_json,
                 created_at_ms, updated_at_ms
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                 ?18, ?19, ?20, ?21, ?22
             )",
            params![
                platform_id,
                company_id,
                project_id,
                task.id.as_str(),
                task_kind_key(task.kind),
                task.status.as_str(),
                task.title,
                task.spec_uri,
                task.meta_uri,
                task.assigned_to.as_ref().map(AssigneeId::as_str),
                task.team_id.as_ref().map(TeamId::as_str),
                task.room_id,
                task.project_room_id,
                task.delegated_to_team.as_ref().map(TeamId::as_str),
                task.schedule,
                task.timezone,
                task.next_scheduled_at,
                task.last_executed_at,
                dag_json,
                metadata_json,
                task.created_at_ms.0,
                task.created_at_ms.0,
            ],
        )?;
        let stored = load_task_with_conn(&tx, context, &task.id)?
            .with_context(|| format!("task disappeared after create: {}", task.id.as_str()))?;
        tx.commit()?;

        if inserted == 0 {
            return Ok(stored);
        }
        Ok(stored)
    }

    pub fn load_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Option<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_task(context, task_id);
        }
        let conn = open_initialized_connection(self, context)?;
        load_task_with_conn(&conn, context, task_id)
    }

    pub fn list_active_tasks(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
    ) -> Result<Vec<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_active_tasks(context, filter);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut query = format!(
            "SELECT {TASK_COLUMNS}
             FROM scheduler_tasks
             WHERE platform_id = ? AND company_id = ? AND project_id = ?
               AND status NOT IN ('completed', 'cancelled', 'archived')"
        );
        let mut bind_values =
            vec![platform_id.to_string(), company_id.to_string(), project_id.to_string()];

        if let Some(status) = filter.status {
            query.push_str(" AND status = ?");
            bind_values.push(status.as_str().to_string());
        }
        if let Some(worker) = &filter.assigned_to {
            query.push_str(" AND assigned_to = ?");
            bind_values.push(worker.as_str().to_string());
        }
        if let Some(task_id) = &filter.task_id {
            query.push_str(" AND id = ?");
            bind_values.push(task_id.as_str().to_string());
        }
        query.push_str(" ORDER BY status, id");

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params_from_iter(bind_values.iter()), map_task_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().context("list active agent tasks")
    }

    pub fn list_tasks(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
    ) -> Result<Vec<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_tasks(context, filter);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut query = format!(
            "SELECT {TASK_COLUMNS}
             FROM scheduler_tasks
             WHERE platform_id = ? AND company_id = ? AND project_id = ?"
        );
        let mut bind_values =
            vec![platform_id.to_string(), company_id.to_string(), project_id.to_string()];
        if let Some(status) = filter.status {
            query.push_str(" AND status = ?");
            bind_values.push(status.as_str().to_string());
        }
        if let Some(worker) = &filter.assigned_to {
            query.push_str(" AND assigned_to = ?");
            bind_values.push(worker.as_str().to_string());
        }
        if let Some(task_id) = &filter.task_id {
            query.push_str(" AND id = ?");
            bind_values.push(task_id.as_str().to_string());
        }
        query.push_str(" ORDER BY id");
        let mut statement = conn.prepare(&query)?;
        statement
            .query_map(params_from_iter(bind_values.iter()), map_task_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("list scoped agent tasks")
    }

    pub fn list_task_board_tasks(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
    ) -> Result<Vec<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_task_board_tasks(context, filter);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let mut query = format!(
            "SELECT {TASK_COLUMNS}
             FROM scheduler_tasks
             WHERE platform_id = ? AND company_id = ? AND project_id = ?
               AND status NOT IN ('cancelled', 'archived')"
        );
        let mut bind_values =
            vec![platform_id.to_string(), company_id.to_string(), project_id.to_string()];

        if let Some(status) = filter.status {
            query.push_str(" AND status = ?");
            bind_values.push(status.as_str().to_string());
        }
        if let Some(worker) = &filter.assigned_to {
            query.push_str(" AND assigned_to = ?");
            bind_values.push(worker.as_str().to_string());
        }
        if let Some(task_id) = &filter.task_id {
            query.push_str(" AND id = ?");
            bind_values.push(task_id.as_str().to_string());
        }
        query.push_str(" ORDER BY status, id");

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params_from_iter(bind_values.iter()), map_task_row)?;
        let mut tasks =
            rows.collect::<rusqlite::Result<Vec<_>>>().context("list task board agent tasks")?;
        tasks.retain(|task| !bool_metadata_field(&task.metadata, "task_board.deleted"));
        tasks.sort_by(|left, right| task_board_sort_key(left).cmp(&task_board_sort_key(right)));
        Ok(tasks)
    }

    pub fn update_task_board_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        update: AgentTaskBoardUpdate,
    ) -> Result<Option<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.update_task_board_task(context, task_id, update);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(current) = load_task_with_conn(&tx, context, task_id)? else {
            return Ok(None);
        };
        let title = match update.title {
            Some(title) => {
                ensure_non_empty(&title, "task title")?;
                title
            }
            None => current.title,
        };
        let status = update.status.unwrap_or(current.status);
        let metadata_json =
            serde_json::to_string(&update.metadata).context("serialize task metadata")?;
        tx.execute(
            "UPDATE scheduler_tasks
             SET title = ?1, status = ?2, metadata_json = ?3, updated_at_ms = ?4
             WHERE platform_id = ?5 AND company_id = ?6 AND project_id = ?7 AND id = ?8",
            params![
                title,
                status.as_str(),
                metadata_json,
                update.updated_at_ms.0,
                platform_id,
                company_id,
                project_id,
                task_id.as_str(),
            ],
        )?;
        let updated = load_task_with_conn(&tx, context, task_id)?.with_context(|| {
            format!("task disappeared after task board update: {}", task_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(updated))
    }

    /// 按状态机约束转换任务状态；非法边不会写入数据库。
    pub fn transition_task_status(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        to_status: AgentTaskStatus,
        updated_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.transition_task_status(context, task_id, to_status, updated_at_ms);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(current) = load_task_with_conn(&tx, context, task_id)? else {
            return Ok(None);
        };
        ensure_transition(current.kind, current.status, to_status)?;
        let metadata = task_board_projection_metadata(current.metadata, to_status);
        let metadata_json = serde_json::to_string(&metadata).context("serialize task metadata")?;
        tx.execute(
            "UPDATE scheduler_tasks SET status = ?1, metadata_json = ?2, updated_at_ms = ?3
             WHERE platform_id = ?4 AND company_id = ?5 AND project_id = ?6 AND id = ?7",
            params![
                to_status.as_str(),
                metadata_json,
                updated_at_ms.0,
                platform_id,
                company_id,
                project_id,
                task_id.as_str()
            ],
        )?;
        let updated = load_task_with_conn(&tx, context, task_id)?.with_context(|| {
            format!("task disappeared after status transition: {}", task_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(updated))
    }

    /// 在同一事务中转换状态并替换元数据，避免看板投影与状态分离。
    pub fn transition_task_status_with_metadata(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        to_status: AgentTaskStatus,
        metadata: Value,
        updated_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.transition_task_status_with_metadata(
                context,
                task_id,
                to_status,
                metadata,
                updated_at_ms,
            );
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(current) = load_task_with_conn(&tx, context, task_id)? else {
            return Ok(None);
        };
        ensure_transition(current.kind, current.status, to_status)?;
        let metadata_json = serde_json::to_string(&metadata).context("serialize task metadata")?;
        tx.execute(
            "UPDATE scheduler_tasks SET status = ?1, metadata_json = ?2, updated_at_ms = ?3
             WHERE platform_id = ?4 AND company_id = ?5 AND project_id = ?6 AND id = ?7",
            params![
                to_status.as_str(),
                metadata_json,
                updated_at_ms.0,
                platform_id,
                company_id,
                project_id,
                task_id.as_str()
            ],
        )?;
        let updated = load_task_with_conn(&tx, context, task_id)?.with_context(|| {
            format!("task disappeared after status transition: {}", task_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(updated))
    }

    /// 同时更新周期任务最近执行时间和下一计划时间。
    pub fn update_recurring_task_execution(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        last_executed_at: &str,
        next_scheduled_at: &str,
        updated_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentTask>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.update_recurring_task_execution(
                context,
                task_id,
                last_executed_at,
                next_scheduled_at,
                updated_at_ms,
            );
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(current) = load_task_with_conn(&tx, context, task_id)? else {
            return Ok(None);
        };
        if current.kind != AgentTaskKind::Infinite {
            bail!(
                "recurring execution can only be recorded for infinite task: {}",
                task_id.as_str()
            );
        }
        tx.execute(
            "UPDATE scheduler_tasks
             SET last_executed_at = ?1, next_scheduled_at = ?2, updated_at_ms = ?3
             WHERE platform_id = ?4 AND company_id = ?5 AND project_id = ?6 AND id = ?7",
            params![
                last_executed_at,
                next_scheduled_at,
                updated_at_ms.0,
                platform_id,
                company_id,
                project_id,
                task_id.as_str()
            ],
        )?;
        let updated = load_task_with_conn(&tx, context, task_id)?.with_context(|| {
            format!("task disappeared after recurring update: {}", task_id.as_str())
        })?;
        tx.commit()?;
        Ok(Some(updated))
    }

    pub fn save_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Result<()> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.save_task_board_workflow(context, workflow);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let workflow = normalize_failed_retry_transitions(workflow.clone());
        let workflow_json =
            serde_json::to_string(&workflow).context("serialize task board workflow")?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(platform_id, company_id, project_id, key)
             DO UPDATE SET value = excluded.value",
            params![
                platform_id,
                company_id,
                project_id,
                TASK_BOARD_WORKFLOW_META_KEY,
                workflow_json
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_task_board_workflow(context);
        }
        let conn = open_initialized_connection(self, context)?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let workflow_json = conn
            .query_row(
                "SELECT value FROM scheduler_meta
                 WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND key = ?4",
                params![platform_id, company_id, project_id, TASK_BOARD_WORKFLOW_META_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        workflow_json
            .map(|value| {
                serde_json::from_str(&value)
                    .map(normalize_failed_retry_transitions)
                    .context("parse task board workflow")
            })
            .transpose()
    }

}

pub(super) fn normalize_failed_retry_transitions(
    mut workflow: TaskPoolWorkflowSnapshotDto,
) -> TaskPoolWorkflowSnapshotDto {
    for status in &mut workflow.statuses {
        if status.role != TaskPoolWorkflowStatusRoleDto::Failed
            || status
                .transitions
                .iter()
                .any(|transition| transition.outcome.trim().eq_ignore_ascii_case("retry"))
        {
            continue;
        }
        let Some(target) = status.next.clone() else {
            continue;
        };
        status.transitions.push(TaskPoolWorkflowTransitionDto {
            outcome: "retry".to_string(),
            label: Some("重试".to_string()),
            target,
        });
    }
    workflow
}

fn validate_new_task(task: &NewAgentTask) -> Result<()> {
    ensure_non_empty(&task.title, "task title")?;
    ensure_non_empty(&task.spec_uri, "task spec uri")?;
    ensure_non_empty(&task.meta_uri, "task meta uri")?;
    Ok(())
}

fn load_task_with_conn(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: &TaskId,
) -> Result<Option<StoredAgentTask>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM scheduler_tasks
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, task_id.as_str()], map_task_row)
        .optional()
        .with_context(|| format!("load agent task: {}", task_id.as_str()))
}

fn map_task_row(row: &Row<'_>) -> rusqlite::Result<StoredAgentTask> {
    let kind_key: String = row.get(1)?;
    let status_key: String = row.get(2)?;
    let assigned_to: Option<String> = row.get(6)?;
    let team_id: Option<String> = row.get(7)?;
    let delegated_to_team: Option<String> = row.get(10)?;
    let dag_json: String = row.get(15)?;
    let metadata_json: String = row.get(16)?;

    Ok(StoredAgentTask {
        id: parse_task_id(row.get(0)?)?,
        kind: AgentTaskKind::parse(&kind_key).map_err(|error| conversion_error(1, error))?,
        status: AgentTaskStatus::parse(&status_key).map_err(|error| conversion_error(2, error))?,
        title: row.get(3)?,
        spec_uri: row.get(4)?,
        meta_uri: row.get(5)?,
        assigned_to: parse_optional_worker_name(assigned_to)?,
        team_id: parse_optional_team_id(team_id)?,
        room_id: row.get(8)?,
        project_room_id: row.get(9)?,
        delegated_to_team: parse_optional_team_id(delegated_to_team)?,
        schedule: row.get(11)?,
        timezone: row.get(12)?,
        next_scheduled_at: row.get(13)?,
        last_executed_at: row.get(14)?,
        dag: parse_dag(&dag_json)?,
        metadata: serde_json::from_str(&metadata_json)
            .map_err(|error| conversion_error(16, anyhow::Error::new(error)))?,
        created_at_ms: TimestampMs(row.get(17)?),
        updated_at_ms: TimestampMs(row.get(18)?),
    })
}

fn parse_task_id(value: String) -> rusqlite::Result<TaskId> {
    TaskId::new(value).map_err(|error| conversion_error(0, error))
}

fn parse_optional_worker_name(value: Option<String>) -> rusqlite::Result<Option<AssigneeId>> {
    value.map(AssigneeId::new).transpose().map_err(|error| conversion_error(6, error))
}

fn parse_optional_team_id(value: Option<String>) -> rusqlite::Result<Option<TeamId>> {
    value.map(TeamId::new).transpose().map_err(|error| conversion_error(7, error))
}

pub(super) fn serialize_dag(nodes: &[AgentTaskDagNode]) -> Result<String> {
    let json_nodes = nodes.iter().map(DagNodeJson::from).collect::<Vec<_>>();
    serde_json::to_string(&json_nodes).context("serialize task DAG nodes")
}

fn parse_dag(value: &str) -> rusqlite::Result<Vec<AgentTaskDagNode>> {
    parse_dag_value(value).map_err(|error| conversion_error(15, error))
}

pub(crate) fn parse_dag_value(value: &str) -> Result<Vec<AgentTaskDagNode>> {
    let json_nodes = serde_json::from_str::<Vec<DagNodeJson>>(value).map_err(anyhow::Error::new)?;
    json_nodes.into_iter().map(AgentTaskDagNode::try_from).collect::<Result<Vec<_>>>()
}

pub(super) fn task_kind_key(kind: AgentTaskKind) -> &'static str {
    match kind {
        AgentTaskKind::Finite => "finite",
        AgentTaskKind::Infinite => "infinite",
        AgentTaskKind::DependencyGraph => "dependency_graph",
    }
}

pub(super) fn ensure_non_empty(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field_name} must not be empty");
    }
    Ok(())
}

pub(super) fn task_board_sort_key(
    task: &StoredAgentTask,
) -> (String, u32, i64, TimestampMs, String) {
    (
        task_board_status_key(task),
        u32_metadata_field(&task.metadata, "priority").unwrap_or_default(),
        i64_metadata_field(&task.metadata, "order_no").unwrap_or_default(),
        task.updated_at_ms,
        task.id.as_str().to_string(),
    )
}

fn task_board_status_key(task: &StoredAgentTask) -> String {
    task_board_status_key_from_metadata(task.status, &task.metadata)
        .unwrap_or_else(|| legacy_task_board_status_key(task.status).to_string())
}

fn task_board_status_key_from_metadata(
    status: AgentTaskStatus,
    metadata: &Value,
) -> Option<String> {
    if agent_scheduler_control_status(status) {
        return task_board_status_key_from_map_value(AgentTaskStatus::Queued, metadata)
            .or_else(|| Some(legacy_task_board_status_key(AgentTaskStatus::Queued).to_string()));
    }
    if let Some(status_key) =
        metadata.get("task_board.status_key").and_then(Value::as_str).and_then(non_empty_str)
    {
        return Some(status_key.to_string());
    }
    task_board_status_key_from_map_value(status, metadata)
}

fn task_board_status_key_from_map_value(
    status: AgentTaskStatus,
    metadata: &Value,
) -> Option<String> {
    metadata
        .get("task_board.status_key_map")
        .and_then(Value::as_object)?
        .get(status.as_str())
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .map(str::to_string)
}

pub(crate) fn task_board_projection_metadata(metadata: Value, status: AgentTaskStatus) -> Value {
    let Value::Object(mut object) = metadata else {
        return metadata;
    };
    let fallback_status = task_board_projection_status(status);
    let status_key = task_board_status_key_from_map(status, &object)
        .unwrap_or_else(|| legacy_task_board_status_key(fallback_status).to_string());
    object.insert("task_board.status_key".to_string(), json!(status_key));
    object.insert("task_board.status_role".to_string(), json!(task_board_status_role(status)));
    Value::Object(object)
}

fn task_board_status_key_from_map(
    status: AgentTaskStatus,
    metadata: &Map<String, Value>,
) -> Option<String> {
    let status = task_board_projection_status(status);
    metadata
        .get("task_board.status_key_map")
        .and_then(Value::as_object)?
        .get(status.as_str())
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .map(str::to_string)
}

fn legacy_task_board_status_key(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Submitted => "code_complete",
        _ => status.as_str(),
    }
}

fn task_board_status_role(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Draft => "backlog",
        AgentTaskStatus::Queued
        | AgentTaskStatus::Active
        | AgentTaskStatus::Assigned
        | AgentTaskStatus::Dispatched
        | AgentTaskStatus::Acknowledged => "queued",
        AgentTaskStatus::Running => "executing",
        AgentTaskStatus::Submitted => "review_ready",
        AgentTaskStatus::Revision => "review",
        AgentTaskStatus::Blocked | AgentTaskStatus::Paused => "paused",
        AgentTaskStatus::Completed => "done",
        AgentTaskStatus::Failed => "failed",
        AgentTaskStatus::Cancelled | AgentTaskStatus::Archived => "archived",
    }
}

fn agent_scheduler_control_status(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned | AgentTaskStatus::Dispatched | AgentTaskStatus::Acknowledged
    )
}

fn task_board_projection_status(status: AgentTaskStatus) -> AgentTaskStatus {
    if agent_scheduler_control_status(status) { AgentTaskStatus::Queued } else { status }
}

pub(super) fn bool_metadata_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or_default()
}

fn u32_metadata_field(value: &Value, key: &str) -> Option<u32> {
    value.get(key).and_then(Value::as_u64).and_then(|value| u32::try_from(value).ok())
}

fn i64_metadata_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}

#[derive(Debug, Serialize, Deserialize)]
struct DagNodeJson {
    task_id: String,
    title: String,
    #[serde(default)]
    depends_on: Vec<String>,
    assigned_to: Option<String>,
    status: String,
}

impl From<&AgentTaskDagNode> for DagNodeJson {
    fn from(value: &AgentTaskDagNode) -> Self {
        Self {
            task_id: value.task_id.as_str().to_string(),
            title: value.title.clone(),
            depends_on: value
                .depends_on
                .iter()
                .map(|task_id| task_id.as_str().to_string())
                .collect(),
            assigned_to: value.assigned_to.as_ref().map(AssigneeId::as_str).map(str::to_string),
            status: value.status.as_str().to_string(),
        }
    }
}

impl TryFrom<DagNodeJson> for AgentTaskDagNode {
    type Error = anyhow::Error;

    fn try_from(value: DagNodeJson) -> Result<Self> {
        Ok(Self {
            task_id: TaskId::new(value.task_id)?,
            title: value.title,
            depends_on: value
                .depends_on
                .into_iter()
                .map(TaskId::new)
                .collect::<Result<Vec<_>>>()?,
            assigned_to: value.assigned_to.map(AssigneeId::new).transpose()?,
            status: AgentTaskStatus::parse(&value.status)?,
        })
    }
}
