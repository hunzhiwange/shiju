//! 实现 SQLite 任务与看板数据的创建、读取、筛选和受状态机约束的更新。

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskLabelDto, AgentSchedulerTaskPropertyChanges, AgentSchedulerTaskPropertyDto,
    AgentSchedulerTaskPropertyKind, AgentSchedulerTaskPropertyValue, AgentSchedulerTaskPropertyValues,
    TASK_LABEL_IDS_METADATA_KEY, TASK_PROPERTY_VALUES_METADATA_KEY,
};
use vw_api_transport::common::TimestampMs;
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use crate::domain::{AssigneeId, TaskId, TeamId};
use crate::lifecycle_bridge::{
    LifecycleBridge, normalize_workflow_transitions, runtime_status_for_role, task_workflow_snapshot,
};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus, ensure_transition};

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    open_locked_initialized_connection, postgres_provider, scope_params,
};

use super::events::insert_event_with_conn;
use super::task_plan::{
    PlacementWrite, TaskPlanEdit, TaskPlanPlacement, became_hidden, load_plan_members, placement_event,
    sqlite_publication_exists, validate_hiding, validate_metadata_completion, validate_summary_child_update, write_placement,
};

const TASK_COLUMNS: &str = "id, kind, status, title, spec_uri, meta_uri, assigned_to, team_id, \
room_id, project_room_id, delegated_to_team, schedule, timezone, next_scheduled_at, \
last_executed_at, dag_json, metadata_json, created_at_ms, updated_at_ms";
const TASK_BOARD_WORKFLOW_META_KEY: &str = "task_board.workflow";
const TASK_LABELS_META_KEY: &str = "task_board.labels";
pub(super) const TASK_PROPERTIES_META_KEY: &str = "task_board.properties";

pub use task_scheduler_core::task_records::{NewAgentTask, StoredAgentTask, AgentTaskDagNode};

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

#[derive(Debug)]
pub enum WorkflowSelectionError {
    Invalid(String),
    NotFound(String),
    Conflict(String),
    Storage(anyhow::Error),
}

impl AgentSchedulerStore {
    /// 将租户预设固定为项目快照，保留注册表后续编辑前的已生效定义。
    pub async fn select_task_board_workflow(
        &self,
        context: &crate::domain::AgentSchedulerContext,
        workflow_id: String,
        expected: Option<TaskPoolWorkflowSnapshotDto>,
    ) -> std::result::Result<TaskPoolWorkflowSnapshotDto, WorkflowSelectionError> {
        let tenant = context.scope().project_scope().tenant_scope().clone();
        let workflow = task_lifecycle::store::workflow_by_id(
            self.database().map_err(WorkflowSelectionError::Storage)?,
            tenant,
            workflow_id,
        )
        .await
        .map_err(|error| match error {
            task_lifecycle::TaskLifecycleError::BadRequest(message) => WorkflowSelectionError::Invalid(message),
            task_lifecycle::TaskLifecycleError::NotFound(message) => WorkflowSelectionError::NotFound(message),
            task_lifecycle::TaskLifecycleError::Internal(message) => WorkflowSelectionError::Storage(anyhow::anyhow!(message)),
        })?;
        let workflow = normalize_workflow_transitions(task_workflow_snapshot(workflow));
        workflow.validate_for_schedule().map_err(WorkflowSelectionError::Invalid)?;
        let conflict = self
            .replace_task_board_workflow(context, expected.as_ref(), &workflow)
            .map_err(WorkflowSelectionError::Storage)?;
        if let Some(message) = conflict {
            return Err(WorkflowSelectionError::Conflict(message));
        }
        Ok(workflow)
    }

    pub(crate) fn replace_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        expected: Option<&TaskPoolWorkflowSnapshotDto>,
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Result<Option<String>> {
        let workflow = normalize_workflow_transitions(workflow.clone());
        if let Some(store) = postgres_provider(self, context)? {
            return store.replace_task_board_workflow(context, expected, &workflow);
        }
        let scope = context.storage_scope()?;
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = load_workflow_with_conn(&tx, &scope)?;
        if let Some(conflict) = workflow_changed(current.as_ref(), expected) {
            return Ok(Some(conflict));
        }
        if current.as_ref() == Some(&workflow) {
            return Ok(None);
        }

        let (platform_id, company_id, project_id) = scope_params(&scope);
        let tasks = {
            let mut statement = tx.prepare(&format!(
                "SELECT {TASK_COLUMNS} FROM scheduler_tasks
                 WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
                   AND status NOT IN ('cancelled', 'archived')"
            ))?;
            let rows = statement.query_map(params![platform_id, company_id, project_id], map_task_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        if let Some(conflict) = workflow_task_conflicts(&tasks, &workflow) {
            return Ok(Some(conflict));
        }

        upsert_workflow_with_conn(&tx, &scope, &workflow)?;
        tx.commit()?;
        Ok(None)
    }

    /// 在作用域内创建唯一任务，并返回规范化后的持久化快照。
    pub fn create_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task: NewAgentTask,
    ) -> Result<StoredAgentTask> {
        self.create_task_with_placement(context, task, None)
    }

    pub(super) fn create_task_with_placement(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        mut task: NewAgentTask,
        placement: Option<TaskPlanPlacement>,
    ) -> Result<StoredAgentTask> {
        validate_new_task(&task)?;
        if task.metadata.get(crate::task_board::detail::COLLABORATION_KEY).is_some() {
            bail!("use the task detail API to change collaboration data");
        }
        if let Some(metadata) = task.metadata.as_object() {
            for key in metadata.keys() {
                crate::task_board::validate_task_plan_metadata_key(key)?;
            }
        }
        if task.metadata.get(TASK_PROPERTY_VALUES_METADATA_KEY).is_some() {
            bail!("use property_changes to set task properties");
        }
        if let Some(value) = task.metadata.get(TASK_LABEL_IDS_METADATA_KEY) {
            let ids = serde_json::from_value(value.clone()).context("invalid task label IDs")?;
            let labels = self.load_task_labels(context)?;
            let ids = validate_task_label_ids(ids, &labels, &Value::Null)?;
            task.metadata[TASK_LABEL_IDS_METADATA_KEY] = json!(ids);
        }
        if let Some(store) = postgres_provider(self, context)? {
            return store.create_task(context, task, placement);
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
        if let Some(placement) = placement.as_ref() {
            let mode = if inserted == 0 {
                PlacementWrite::RetryCreate
            } else {
                PlacementWrite::Create
            };
            if write_placement(&tx, context, &scope, &stored, Some(placement), mode)? {
                insert_event_with_conn(
                    &tx,
                    &scope,
                    placement_event(&stored, Some(placement), true),
                )?;
            }
        }
        tx.commit()?;

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
        tasks.sort_by_key(task_board_sort_key);
        Ok(tasks)
    }

    pub fn update_task_board_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        mut update: AgentTaskBoardUpdate,
    ) -> Result<Option<StoredAgentTask>> {
        let outcome = self.edit_task_board_task(context, task_id, BTreeMap::new(), |current| {
            preserve_task_board_fields(&current.metadata, &mut update.metadata);
            Ok(update)
        })?;
        Ok(outcome.map(|(_, updated)| updated))
    }

    /// Apply a partial edit to the row locked by the write transaction.
    pub(crate) fn edit_task_board_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        property_changes: AgentSchedulerTaskPropertyChanges,
        edit: impl FnOnce(&StoredAgentTask) -> Result<AgentTaskBoardUpdate> + Send,
    ) -> Result<Option<(StoredAgentTask, StoredAgentTask)>> {
        self.edit_task_with_placement(context, task_id, property_changes, TaskPlanEdit::Preserve, edit)
    }

    pub(crate) fn edit_task_board_metadata(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        property_changes: AgentSchedulerTaskPropertyChanges,
        edit: impl FnOnce(&StoredAgentTask) -> Result<AgentTaskBoardUpdate> + Send,
    ) -> Result<Option<(StoredAgentTask, StoredAgentTask)>> {
        self.edit_task_with_placement(context, task_id, property_changes, TaskPlanEdit::Metadata, edit)
    }

    pub(super) fn edit_task_with_placement(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        property_changes: AgentSchedulerTaskPropertyChanges,
        plan_edit: TaskPlanEdit,
        edit: impl FnOnce(&StoredAgentTask) -> Result<AgentTaskBoardUpdate> + Send,
    ) -> Result<Option<(StoredAgentTask, StoredAgentTask)>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.edit_task_with_placement(context, task_id, property_changes, plan_edit, edit);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(current) = load_task_with_conn(&tx, context, task_id)? else {
            return Ok(None);
        };
        let mut update = edit(&current)?;
        if !property_changes.is_empty() {
            let properties = parse_directory(load_meta(&tx, &scope, TASK_PROPERTIES_META_KEY)?)?;
            apply_task_property_changes(&mut update.metadata, &properties, property_changes)?;
        }
        let title = match update.title {
            Some(title) => {
                ensure_non_empty(&title, "task title")?;
                title
            }
            None => current.title.clone(),
        };
        let status = update.status.unwrap_or(current.status);
        validate_summary_child_update(&tx, &scope, &current, status, &update.metadata)?;
        let hiding = became_hidden(&current, status, &update.metadata);
        if hiding {
            let archived_published_parent = status == AgentTaskStatus::Archived
                && !bool_metadata_field(&update.metadata, "task_board.deleted")
                && sqlite_publication_exists(&tx, &scope, task_id)?;
            validate_hiding(
                &load_plan_members(&tx, &scope, task_id)?, task_id, archived_published_parent,
            )?;
        }
        if matches!(&plan_edit, TaskPlanEdit::Metadata)
            && status == AgentTaskStatus::Completed
            && (current.status != status
                || crate::task_board::task_board_status_key(&current)
                    != crate::task_board::task_board_status_key_for(status, &update.metadata))
        {
            validate_metadata_completion(&load_plan_members(&tx, &scope, task_id)?, task_id)?;
        }
        let placement_changed = if let TaskPlanEdit::Replace(replacement) = &plan_edit {
            let changed = write_placement(
                &tx,
                context,
                &scope,
                &current,
                replacement.as_ref(),
                PlacementWrite::Edit,
            )?;
            if changed && hiding {
                bail!("cannot change placement while hiding task {}", task_id.as_str());
            }
            changed
        } else {
            false
        };
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
        if placement_changed && let TaskPlanEdit::Replace(replacement) = &plan_edit {
            insert_event_with_conn(
                &tx,
                &scope,
                placement_event(&updated, replacement.as_ref(), false),
            )?;
        }
        tx.commit()?;
        Ok(Some((current, updated)))
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
        let pausing_plan = to_status == AgentTaskStatus::Paused
            && matches!(current.status, AgentTaskStatus::Blocked | AgentTaskStatus::Queued)
            && sqlite_publication_exists(&tx, &scope, task_id)?;
        if !pausing_plan {
            ensure_transition(current.kind, current.status, to_status)?;
        }
        let metadata = task_board_projection_metadata(current.metadata.clone(), to_status);
        validate_summary_child_update(&tx, &scope, &current, to_status, &metadata)?;
        if became_hidden(&current, to_status, &metadata) {
            let archived_published_parent = to_status == AgentTaskStatus::Archived
                && !bool_metadata_field(&metadata, "task_board.deleted")
                && sqlite_publication_exists(&tx, &scope, task_id)?;
            validate_hiding(
                &load_plan_members(&tx, &scope, task_id)?, task_id, archived_published_parent,
            )?;
        }
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
        mut metadata: Value,
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
        preserve_task_board_fields(&current.metadata, &mut metadata);
        validate_summary_child_update(&tx, &scope, &current, to_status, &metadata)?;
        if became_hidden(&current, to_status, &metadata) {
            let archived_published_parent = to_status == AgentTaskStatus::Archived
                && !bool_metadata_field(&metadata, "task_board.deleted")
                && sqlite_publication_exists(&tx, &scope, task_id)?;
            validate_hiding(
                &load_plan_members(&tx, &scope, task_id)?, task_id, archived_published_parent,
            )?;
        }
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
        let (_guard, conn) = open_locked_initialized_connection(self, context)?;
        let workflow = normalize_workflow_transitions(workflow.clone());
        upsert_workflow_with_conn(&conn, &scope, &workflow)
    }

    pub fn load_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_task_board_workflow(context);
        }
        let conn = open_initialized_connection(self, context)?;
        load_workflow_with_conn(&conn, &context.storage_scope()?)
    }

    pub fn load_task_labels(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<AgentSchedulerTaskLabelDto>> {
        self.load_task_directory(context, TASK_LABELS_META_KEY)
    }

    pub fn edit_task_labels(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        edit: impl FnOnce(&mut Vec<AgentSchedulerTaskLabelDto>) -> Result<()> + Send,
    ) -> Result<Vec<AgentSchedulerTaskLabelDto>> {
        self.edit_task_directory(context, TASK_LABELS_META_KEY, edit)
    }

    pub fn load_task_properties(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<AgentSchedulerTaskPropertyDto>> {
        self.load_task_directory(context, TASK_PROPERTIES_META_KEY)
    }

    pub fn edit_task_properties(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        edit: impl FnOnce(&mut Vec<AgentSchedulerTaskPropertyDto>) -> Result<()> + Send,
    ) -> Result<Vec<AgentSchedulerTaskPropertyDto>> {
        self.edit_task_directory(context, TASK_PROPERTIES_META_KEY, edit)
    }

    fn load_task_directory<T: serde::de::DeserializeOwned>(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        key: &str,
    ) -> Result<Vec<T>> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_task_directory(&scope, key);
        }
        let conn = open_initialized_connection(self, context)?;
        parse_directory(load_meta(&conn, &scope, key)?)
    }

    fn edit_task_directory<T: serde::de::DeserializeOwned + Serialize + Send>(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        key: &str,
        edit: impl FnOnce(&mut Vec<T>) -> Result<()> + Send,
    ) -> Result<Vec<T>> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.edit_task_directory(&scope, key, edit);
        }
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut directory = parse_directory(load_meta(&tx, &scope, key)?)?;
        edit(&mut directory)?;
        upsert_meta(&tx, &scope, key, &serde_json::to_string(&directory)?)?;
        tx.commit()?;
        Ok(directory)
    }
}

pub(super) fn parse_directory<T: serde::de::DeserializeOwned>(value: Option<String>) -> Result<Vec<T>> {
    value.map(|value| serde_json::from_str(&value).context("read task directory"))
        .transpose()
        .map(Option::unwrap_or_default)
}

pub(super) fn apply_task_property_changes(
    metadata: &mut Value,
    directory: &[AgentSchedulerTaskPropertyDto],
    changes: AgentSchedulerTaskPropertyChanges,
) -> Result<()> {
    use AgentSchedulerTaskPropertyKind as Kind;
    use AgentSchedulerTaskPropertyValue as Property;

    let mut values: AgentSchedulerTaskPropertyValues = metadata
        .get(TASK_PROPERTY_VALUES_METADATA_KEY)
        .map(|value| serde_json::from_value(value.clone()).context("read task property values"))
        .transpose()?
        .unwrap_or_default();
    for (id, mut value) in changes {
        let field = directory.iter().find(|field| field.id == id)
            .context("task property does not belong to this project")?;
        if field.archived {
            bail!("task property is archived");
        }
        match (value.as_mut(), field.kind) {
            (None, _) => {
                values.remove(&id);
            }
            (Some(Property::Date(date)), Kind::Date) => {
                let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .context("task property date must be a valid date in YYYY-MM-DD format")?;
                if date.len() != 10 || date.starts_with("0000") || parsed.to_string() != *date {
                    bail!("task property date must be a valid date in YYYY-MM-DD format");
                }
            }
            (Some(Property::SingleSelect(option)), Kind::SingleSelect) => {
                let choice = field.options.iter().find(|choice| choice.id == *option)
                    .context("option does not belong to this task property")?;
                if choice.archived {
                    bail!("task property option is archived");
                }
            }
            (Some(Property::MultiSelect(options)), Kind::MultiSelect) => {
                let previous = match values.get(&id) {
                    Some(Property::MultiSelect(previous)) => previous.as_slice(),
                    _ => &[],
                };
                options.sort_unstable();
                options.dedup();
                for option in options {
                    let choice = field.options.iter().find(|choice| &choice.id == option)
                        .context("option does not belong to this task property")?;
                    if choice.archived && !previous.contains(option) {
                        bail!("task property option is archived");
                    }
                }
            }
            (Some(Property::Number(number)), Kind::Number) => {
                *number = number.trim().parse::<serde_json::Number>()
                    .context("task property number must be a finite number")?.to_string();
            }
            (Some(Property::Checkbox(_)), Kind::Checkbox) => {}
            (Some(Property::Link(link)), Kind::Link) => validate_task_property_link(link)?,
            (Some(Property::Member(member)), Kind::Member) => validate_task_property_member(member)?,
            (Some(Property::MultiMember(members)), Kind::MultiMember) => {
                for member in members.iter_mut() {
                    validate_task_property_member(member)?;
                }
                let mut seen = std::collections::HashSet::new();
                members.retain(|member| seen.insert(member.clone()));
            }
            (Some(Property::TextList(items)), Kind::TextList) => {
                for item in items {
                    if item.trim().is_empty() {
                        bail!("task property list items must not be empty");
                    }
                }
            }
            (Some(Property::LinkList(links)), Kind::LinkList) => {
                for link in links {
                    validate_task_property_link(link)?;
                }
            }
            (Some(Property::Text(_)), Kind::Text) => {}
            _ => bail!("value does not match task property type"),
        }
        if let Some(value) = value {
            values.insert(id, value);
        }
    }
    let metadata = metadata.as_object_mut().context("task metadata must be an object")?;
    if values.is_empty() {
        metadata.remove(TASK_PROPERTY_VALUES_METADATA_KEY);
    } else {
        metadata.insert(TASK_PROPERTY_VALUES_METADATA_KEY.to_owned(), serde_json::to_value(values)?);
    }
    Ok(())
}

fn validate_task_property_member(member: &mut String) -> Result<()> {
    let id = member.trim();
    if id.is_empty() || id.chars().any(char::is_whitespace) {
        bail!("task property member must be a member ID");
    }
    *member = id.to_owned();
    Ok(())
}

fn validate_task_property_link(link: &mut String) -> Result<()> {
    let value = link.trim();
    let url = url::Url::parse(value).context("task property link must be a valid URL")?;
    if !matches!(url.scheme(), "http" | "https") || !url.has_host()
        || !url.username().is_empty() || url.password().is_some()
    {
        bail!("task property link must be an HTTP or HTTPS URL without credentials");
    }
    *link = value.to_owned();
    Ok(())
}

pub(crate) fn validate_task_label_ids(
    mut ids: Vec<String>,
    labels: &[AgentSchedulerTaskLabelDto],
    current: &Value,
) -> Result<Vec<String>> {
    ids.sort_unstable();
    ids.dedup();
    let current = current.get(TASK_LABEL_IDS_METADATA_KEY).and_then(Value::as_array);
    for id in &ids {
        let label = labels.iter().find(|label| &label.id == id)
            .with_context(|| format!("task label does not belong to this project: {id}"))?;
        if label.archived && !current.is_some_and(|ids| {
            ids.iter().any(|existing| existing.as_str() == Some(id.as_str()))
        }) {
            bail!("task label is archived: {id}");
        }
    }
    Ok(ids)
}

pub(super) fn preserve_task_board_fields(current: &Value, metadata: &mut Value) {
    for key in [
        TASK_LABEL_IDS_METADATA_KEY,
        TASK_PROPERTY_VALUES_METADATA_KEY,
        crate::task_board::detail::COLLABORATION_KEY,
        crate::task_board::detail::CREATOR_KEY,
        "assignee",
    ] {
        if let Some(value) = current.get(key) {
            if !metadata.is_object() {
                *metadata = json!({});
            }
            metadata[key] = value.clone();
        } else if let Some(metadata) = metadata.as_object_mut() {
            metadata.remove(key);
        }
    }
}

fn upsert_workflow_with_conn(
    conn: &Connection,
    scope: &vw_api_agent_scheduler::AgentSchedulerScope,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Result<()> {
    let workflow_json = serde_json::to_string(workflow).context("serialize task board workflow")?;
    upsert_meta(conn, scope, TASK_BOARD_WORKFLOW_META_KEY, &workflow_json)
}

fn upsert_meta(
    conn: &Connection,
    scope: &vw_api_agent_scheduler::AgentSchedulerScope,
    key: &str,
    value: &str,
) -> Result<()> {
    let (platform_id, company_id, project_id) = scope_params(scope);
    conn.execute(
        "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(platform_id, company_id, project_id, key)
         DO UPDATE SET value = excluded.value",
        params![platform_id, company_id, project_id, key, value],
    )?;
    Ok(())
}

pub(super) fn load_workflow_with_conn(
    conn: &Connection,
    scope: &vw_api_agent_scheduler::AgentSchedulerScope,
) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
    parse_saved_workflow(load_meta(conn, scope, TASK_BOARD_WORKFLOW_META_KEY)?)
}

fn load_meta(
    conn: &Connection,
    scope: &vw_api_agent_scheduler::AgentSchedulerScope,
    key: &str,
) -> Result<Option<String>> {
    let (platform_id, company_id, project_id) = scope_params(scope);
    Ok(conn.query_row(
        "SELECT value FROM scheduler_meta
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND key = ?4",
        params![platform_id, company_id, project_id, key],
        |row| row.get(0),
    ).optional()?)
}

pub(super) fn parse_saved_workflow(value: Option<String>) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
    value.map(|value| {
        let workflow: TaskPoolWorkflowSnapshotDto =
            serde_json::from_str(&value).context("parse task board workflow")?;
        Ok(normalize_workflow_transitions(workflow))
    }).transpose()
}

pub(super) fn workflow_changed(
    current: Option<&TaskPoolWorkflowSnapshotDto>,
    expected: Option<&TaskPoolWorkflowSnapshotDto>,
) -> Option<String> {
    (current != expected).then(|| "项目工作流已被修改，请重新打开项目设置后再保存".to_string())
}

pub(super) fn workflow_task_conflicts(
    tasks: &[StoredAgentTask],
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Option<String> {
    let bridge = LifecycleBridge::new(workflow);
    let mut incompatible = BTreeMap::new();
    let mut executing = BTreeMap::new();
    for task in tasks {
        if bool_metadata_field(&task.metadata, "task_board.deleted")
            || bool_metadata_field(&task.metadata, "task_board.archived")
        {
            continue;
        }
        let key = crate::task_board::task_board_status_key(task);
        if agent_scheduler_control_status(task.status) || task.status == AgentTaskStatus::Running {
            *executing.entry(key).or_insert(0) += 1;
        } else if bridge.status(&key).map_or(true, |status| {
            status.role != TaskPoolWorkflowStatusRoleDto::Custom
                && runtime_status_for_role(status.role).is_ok_and(|runtime| runtime != task.status)
        }) {
            *incompatible.entry(key).or_insert(0) += 1;
        }
    }
    let describe = |counts: BTreeMap<String, usize>| {
        counts.into_iter().map(|(key, count)| format!("{key}（{count} 项）"))
            .collect::<Vec<_>>().join("、")
    };
    let mut conflicts = Vec::new();
    if !incompatible.is_empty() {
        conflicts.push(format!("新工作流无法识别的任务状态：{}", describe(incompatible)));
    }
    if !executing.is_empty() {
        conflicts.push(format!("正在执行的任务状态：{}", describe(executing)));
    }
    (!conflicts.is_empty()).then(|| conflicts.join("。"))
}

fn validate_new_task(task: &NewAgentTask) -> Result<()> {
    ensure_non_empty(&task.title, "task title")?;
    ensure_non_empty(&task.spec_uri, "task spec uri")?;
    ensure_non_empty(&task.meta_uri, "task meta uri")?;
    Ok(())
}

pub(super) fn load_task_with_conn(
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
    let (priority, order, updated_at, id) = task_dispatch_sort_key(task);
    (task_board_status_key(task), priority, order, updated_at, id)
}

pub(crate) fn task_dispatch_sort_key(task: &StoredAgentTask) -> (u32, i64, TimestampMs, String) {
    (
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
        AgentTaskStatus::Queued => "pending",
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
