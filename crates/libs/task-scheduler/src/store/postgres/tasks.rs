//! 实现 PostgreSQL 任务与看板状态的创建、筛选和并发安全更新。
//! 状态和元数据更新锁定目标行，并要求恰好一行受影响以暴露并发删除。

use ::postgres::types::ToSql;
use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use vw_api_agent_scheduler::{AgentSchedulerScope, AgentSchedulerTaskPropertyChanges};
use vw_api_transport::common::TimestampMs;
use vw_api_task::TaskPoolWorkflowSnapshotDto;

use super::PostgresAgentSchedulerStore;
use crate::domain::{AssigneeId, TaskId, TeamId};
use crate::lifecycle_bridge::normalize_workflow_transitions;
use crate::state_machine::{AgentTaskKind, AgentTaskStatus, ensure_transition};
use crate::store::AgentSchedulerStorageContext;
use crate::store::tasks::{
    AgentTaskBoardUpdate, AgentTaskFilter, NewAgentTask, StoredAgentTask, TASK_PROPERTIES_META_KEY,
    apply_task_property_changes, bool_metadata_field, ensure_non_empty, parse_dag_value,
    parse_directory, parse_saved_workflow, preserve_task_board_fields, serialize_dag,
    task_board_sort_key, task_kind_key, workflow_changed,
    workflow_task_conflicts,
};

use super::events::insert_event_with_client;
use super::task_plan::{load_plan_members, lock_task_plan_project, write_placement};
use crate::store::task_plan::{
    PlacementWrite, TaskPlanEdit, TaskPlanPlacement, became_hidden, placement_event, validate_hiding,
    validate_metadata_completion,
};

const TASK_COLUMNS: &str = "id, kind, status, title, spec_uri, meta_uri, assigned_to, team_id, \
room_id, project_room_id, delegated_to_team, schedule, timezone, next_scheduled_at, \
last_executed_at, dag_json, metadata_json, created_at_ms, updated_at_ms";
const TASK_BOARD_WORKFLOW_META_KEY: &str = "task_board.workflow";

impl PostgresAgentSchedulerStore {
    pub(in crate::store) fn create_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task: NewAgentTask,
        placement: Option<TaskPlanPlacement>,
    ) -> Result<StoredAgentTask> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let dag_json = serialize_dag(&task.dag)?;
            let metadata_json = serde_json::to_string(&task.metadata)
                .context("serialize PostgreSQL task metadata")?;
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL agent task create")?;
            if placement.is_some() {
                lock_task_plan_project(&mut transaction, &scope)?;
            }
            let inserted = transaction
                .execute(
                    "INSERT INTO scheduler_tasks (
                     platform_id, company_id, project_id, id, kind, status, title, spec_uri,
                     meta_uri, assigned_to, team_id, room_id, project_room_id, delegated_to_team,
                     schedule, timezone, next_scheduled_at, last_executed_at, dag_json,
                     metadata_json, created_at_ms, updated_at_ms
                 ) VALUES (
                     $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                     $16, $17, $18, $19, $20, $21, $22
                 )
                 ON CONFLICT (platform_id, company_id, project_id, id) DO NOTHING",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task.id.as_str(),
                        &task_kind_key(task.kind),
                        &task.status.as_str(),
                        &task.title,
                        &task.spec_uri,
                        &task.meta_uri,
                        &task.assigned_to.as_ref().map(AssigneeId::as_str),
                        &task.team_id.as_ref().map(TeamId::as_str),
                        &task.room_id,
                        &task.project_room_id,
                        &task.delegated_to_team.as_ref().map(TeamId::as_str),
                        &task.schedule,
                        &task.timezone,
                        &task.next_scheduled_at,
                        &task.last_executed_at,
                        &dag_json,
                        &metadata_json,
                        &task.created_at_ms.0,
                        &task.created_at_ms.0,
                    ],
                )
                .with_context(|| format!("create scoped PostgreSQL task: {}", task.id.as_str()))?;
            let stored =
                load_task_with_client(&mut transaction, &scope, &task.id)?.with_context(|| {
                    format!("PostgreSQL task disappeared after create: {}", task.id.as_str())
                })?;
            if let Some(placement) = placement.as_ref() {
                let mode = if inserted == 0 {
                    PlacementWrite::RetryCreate
                } else {
                    PlacementWrite::Create
                };
                if write_placement(&mut transaction, &scope, &stored, Some(placement), mode)? {
                    insert_event_with_client(
                        &mut transaction,
                        &scope,
                        placement_event(&stored, Some(placement), true),
                    )?;
                }
            }
            transaction.commit().context("commit PostgreSQL agent task create")?;
            Ok(stored)
        })
    }

    pub(in crate::store) fn load_task(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Option<StoredAgentTask>> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            load_task_with_client(&mut *self.database.connect_postgres()?, &scope, task_id)
        })
    }

    pub(in crate::store) fn list_active_tasks(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
    ) -> Result<Vec<StoredAgentTask>> {
        self.list_tasks_matching(
            context,
            filter,
            " AND status NOT IN ('completed', 'cancelled', 'archived')",
            "status, id",
        )
    }

    pub(in crate::store) fn list_tasks(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
    ) -> Result<Vec<StoredAgentTask>> {
        self.list_tasks_matching(context, filter, "", "id")
    }

    pub(in crate::store) fn list_task_board_tasks(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
    ) -> Result<Vec<StoredAgentTask>> {
        let mut tasks = self.list_tasks_matching(
            context,
            filter,
            " AND status NOT IN ('cancelled', 'archived')",
            "status, id",
        )?;
        tasks.retain(|task| !bool_metadata_field(&task.metadata, "task_board.deleted"));
        tasks.sort_by(|left, right| task_board_sort_key(left).cmp(&task_board_sort_key(right)));
        Ok(tasks)
    }

    pub(in crate::store) fn edit_task_with_placement(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        property_changes: AgentSchedulerTaskPropertyChanges,
        plan_edit: TaskPlanEdit,
        edit: impl FnOnce(&StoredAgentTask) -> Result<AgentTaskBoardUpdate> + Send,
    ) -> Result<Option<(StoredAgentTask, StoredAgentTask)>> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL task board update")?;
            lock_task_plan_project(&mut transaction, &scope)?;
            let Some(current) = load_task_for_update(&mut transaction, &scope, task_id)? else {
                return Ok(None);
            };
            let mut update = edit(&current)?;
            if !property_changes.is_empty() {
                let properties = parse_directory(load_meta(&mut transaction, &scope, TASK_PROPERTIES_META_KEY)?)?;
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
            let hiding = became_hidden(&current, status, &update.metadata);
            if hiding {
                validate_hiding(&load_plan_members(&mut transaction, &scope, task_id)?, task_id)?;
            }
            if matches!(&plan_edit, TaskPlanEdit::Metadata)
                && status == AgentTaskStatus::Completed
                && (current.status != status
                    || crate::task_board::task_board_status_key(&current)
                        != crate::task_board::task_board_status_key_for(status, &update.metadata))
            {
                validate_metadata_completion(&load_plan_members(&mut transaction, &scope, task_id)?, task_id)?;
            }
            let placement_changed = if let TaskPlanEdit::Replace(replacement) = &plan_edit {
                let changed = write_placement(
                    &mut transaction,
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
            let metadata_json = serde_json::to_string(&update.metadata)
                .context("serialize PostgreSQL task metadata")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_tasks
                 SET title = $1, status = $2, metadata_json = $3, updated_at_ms = $4
                 WHERE platform_id = $5 AND company_id = $6 AND project_id = $7 AND id = $8
                   AND status = $9",
                    &[
                        &title,
                        &status.as_str(),
                        &metadata_json,
                        &update.updated_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                        &current.status.as_str(),
                    ],
                )
                .context("update scoped PostgreSQL task board row")?;
            ensure_single_update(changed, task_id, "task board update")?;
            let updated =
                load_task_with_client(&mut transaction, &scope, task_id)?.with_context(|| {
                    format!("PostgreSQL task disappeared after board update: {}", task_id.as_str())
                })?;
            if placement_changed && let TaskPlanEdit::Replace(replacement) = &plan_edit {
                insert_event_with_client(
                    &mut transaction,
                    &scope,
                    placement_event(&updated, replacement.as_ref(), false),
                )?;
            }
            transaction.commit().context("commit PostgreSQL task board update")?;
            Ok(Some((current, updated)))
        })
    }

    pub(in crate::store) fn transition_task_status(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        to_status: AgentTaskStatus,
        updated_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentTask>> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL task status transition")?;
            lock_task_plan_project(&mut transaction, &scope)?;
            let Some(current) = load_task_for_update(&mut transaction, &scope, task_id)? else {
                return Ok(None);
            };
            ensure_transition(current.kind, current.status, to_status)?;
            let metadata =
                super::super::tasks::task_board_projection_metadata(current.metadata.clone(), to_status);
            if became_hidden(&current, to_status, &metadata) {
                validate_hiding(&load_plan_members(&mut transaction, &scope, task_id)?, task_id)?;
            }
            let metadata_json =
                serde_json::to_string(&metadata).context("serialize PostgreSQL task metadata")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_tasks
                 SET status = $1, metadata_json = $2, updated_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND project_id = $6 AND id = $7
                   AND status = $8",
                    &[
                        &to_status.as_str(),
                        &metadata_json,
                        &updated_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                        &current.status.as_str(),
                    ],
                )
                .context("transition scoped PostgreSQL task status")?;
            ensure_single_update(changed, task_id, "status transition")?;
            let updated =
                load_task_with_client(&mut transaction, &scope, task_id)?.with_context(|| {
                    format!("PostgreSQL task disappeared after transition: {}", task_id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL task status transition")?;
            Ok(Some(updated))
        })
    }

    pub(in crate::store) fn transition_task_status_with_metadata(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        to_status: AgentTaskStatus,
        mut metadata: Value,
        updated_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentTask>> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL task metadata transition")?;
            lock_task_plan_project(&mut transaction, &scope)?;
            let Some(current) = load_task_for_update(&mut transaction, &scope, task_id)? else {
                return Ok(None);
            };
            ensure_transition(current.kind, current.status, to_status)?;
            preserve_task_board_fields(&current.metadata, &mut metadata);
            if became_hidden(&current, to_status, &metadata) {
                validate_hiding(&load_plan_members(&mut transaction, &scope, task_id)?, task_id)?;
            }
            let metadata_json =
                serde_json::to_string(&metadata).context("serialize PostgreSQL task metadata")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_tasks
                 SET status = $1, metadata_json = $2, updated_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND project_id = $6 AND id = $7
                   AND status = $8",
                    &[
                        &to_status.as_str(),
                        &metadata_json,
                        &updated_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                        &current.status.as_str(),
                    ],
                )
                .context("transition scoped PostgreSQL task status with metadata")?;
            ensure_single_update(changed, task_id, "metadata status transition")?;
            let updated =
                load_task_with_client(&mut transaction, &scope, task_id)?.with_context(|| {
                    format!("PostgreSQL task disappeared after transition: {}", task_id.as_str())
                })?;
            transaction.commit().context("commit PostgreSQL task metadata transition")?;
            Ok(Some(updated))
        })
    }

    pub(in crate::store) fn update_recurring_task_execution(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        last_executed_at: &str,
        next_scheduled_at: &str,
        updated_at_ms: TimestampMs,
    ) -> Result<Option<StoredAgentTask>> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL recurring task update")?;
            let Some(current) = load_task_for_update(&mut transaction, &scope, task_id)? else {
                return Ok(None);
            };
            if current.kind != AgentTaskKind::Infinite {
                bail!(
                    "recurring execution can only be recorded for infinite task: {}",
                    task_id.as_str()
                );
            }
            let changed = transaction
                .execute(
                    "UPDATE scheduler_tasks
                 SET last_executed_at = $1, next_scheduled_at = $2, updated_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND project_id = $6 AND id = $7",
                    &[
                        &last_executed_at,
                        &next_scheduled_at,
                        &updated_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &task_id.as_str(),
                    ],
                )
                .context("update scoped PostgreSQL recurring task")?;
            ensure_single_update(changed, task_id, "recurring update")?;
            let updated =
                load_task_with_client(&mut transaction, &scope, task_id)?.with_context(|| {
                    format!(
                        "PostgreSQL task disappeared after recurring update: {}",
                        task_id.as_str()
                    )
                })?;
            transaction.commit().context("commit PostgreSQL recurring task update")?;
            Ok(Some(updated))
        })
    }

    pub(in crate::store) fn save_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Result<()> {
        let scope = context.storage_scope()?;
        let workflow_json =
            serde_json::to_string(&normalize_workflow_transitions(workflow.clone()))
                .context("serialize PostgreSQL task board workflow")?;
        agent_database::run_sync_database(|| {
            let mut connection = self.database.connect_postgres()?;
            let mut tx = connection.transaction()?;
            lock_task_plan_project(&mut tx, &scope)?;
            upsert_meta(&mut tx, &scope, TASK_BOARD_WORKFLOW_META_KEY, &workflow_json)?;
            tx.commit()?;
            Ok(())
        })
    }

    pub(in crate::store) fn load_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
        let scope = context.storage_scope()?;
        parse_saved_workflow(self.load_meta(&scope, TASK_BOARD_WORKFLOW_META_KEY)?)
    }

    pub(in crate::store) fn replace_task_board_workflow(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        expected: Option<&TaskPoolWorkflowSnapshotDto>,
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Result<Option<String>> {
        let scope = context.storage_scope()?;
        let project = scope.project_scope();
        agent_database::run_sync_database(|| {
            let mut connection = self.database.connect_postgres()?;
            let mut tx = connection.transaction()?;
            let platform_id = project.platform_id().as_ref();
            let company_id = project.company_id().as_ref();
            let project_id = project.project_id().as_ref();
            let project_params: &[&(dyn ToSql + Sync)] = &[&platform_id, &company_id, &project_id];
            lock_task_plan_project(&mut tx, &scope)?;
            let current = parse_saved_workflow(tx.query_opt(
                "SELECT value FROM scheduler_meta
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND key = 'task_board.workflow' FOR UPDATE",
                project_params,
            )?.map(|row| row.get(0)))?;
            if let Some(conflict) = workflow_changed(current.as_ref(), expected) {
                return Ok(Some(conflict));
            }
            if current.as_ref() == Some(workflow) {
                return Ok(None);
            }

            let tasks = tx.query(
                &format!("SELECT {TASK_COLUMNS} FROM scheduler_tasks
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND status NOT IN ('cancelled', 'archived') ORDER BY id FOR UPDATE"),
                project_params,
            )?.iter().map(map_task_row).collect::<Result<Vec<_>>>()?;
            if let Some(conflict) = workflow_task_conflicts(&tasks, workflow) {
                return Ok(Some(conflict));
            }

            let workflow_json = serde_json::to_string(workflow)?;
            upsert_meta(&mut tx, &scope, TASK_BOARD_WORKFLOW_META_KEY, &workflow_json)?;
            tx.commit()?;
            Ok(None)
        })
    }

    fn list_tasks_matching(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        filter: AgentTaskFilter,
        fixed_predicate: &str,
        order_by: &str,
    ) -> Result<Vec<StoredAgentTask>> {
        agent_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut query = format!(
                "SELECT {TASK_COLUMNS} FROM scheduler_tasks
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3{fixed_predicate}"
            );
            let mut values: Vec<Box<dyn ToSql + Sync>> = vec![
                Box::new(project.platform_id().as_ref().to_owned()),
                Box::new(project.company_id().as_ref().to_owned()),
                Box::new(project.project_id().as_ref().to_owned()),
            ];
            if let Some(status) = filter.status {
                push_filter(&mut query, &mut values, "status", status.as_str().to_owned());
            }
            if let Some(worker) = filter.assigned_to {
                push_filter(&mut query, &mut values, "assigned_to", worker.as_str().to_owned());
            }
            if let Some(task_id) = filter.task_id {
                push_filter(&mut query, &mut values, "id", task_id.as_str().to_owned());
            }
            query.push_str(&format!(" ORDER BY {order_by}"));
            let parameters = values.iter().map(Box::as_ref).collect::<Vec<_>>();
            self.database
                .connect_postgres()?
                .query(&query, &parameters)
                .context("list scoped PostgreSQL agent tasks")?
                .iter()
                .map(map_task_row)
                .collect()
        })
    }

    pub(in crate::store) fn load_task_directory<T: serde::de::DeserializeOwned>(
        &self,
        scope: &AgentSchedulerScope,
        key: &str,
    ) -> Result<Vec<T>> {
        parse_directory(self.load_meta(scope, key)?)
    }

    pub(in crate::store) fn edit_task_directory<T: serde::de::DeserializeOwned + serde::Serialize + Send>(
        &self,
        scope: &AgentSchedulerScope,
        key: &str,
        edit: impl FnOnce(&mut Vec<T>) -> Result<()> + Send,
    ) -> Result<Vec<T>> {
        agent_database::run_sync_database(|| {
            let mut connection = self.database.connect_postgres()?;
            let mut tx = connection.transaction()?;
            lock_task_plan_project(&mut tx, scope)?;
            let mut directory = parse_directory(load_meta(&mut tx, scope, key)?)?;
            edit(&mut directory)?;
            upsert_meta(&mut tx, scope, key, &serde_json::to_string(&directory)?)?;
            tx.commit()?;
            Ok(directory)
        })
    }

    fn load_meta(&self, scope: &AgentSchedulerScope, key: &str) -> Result<Option<String>> {
        agent_database::run_sync_database(|| {
            load_meta(&mut *self.database.connect_postgres()?, scope, key)
        })
    }
}

pub(super) fn load_workflow_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
) -> Result<Option<TaskPoolWorkflowSnapshotDto>> {
    parse_saved_workflow(load_meta(client, scope, TASK_BOARD_WORKFLOW_META_KEY)?)
}

fn load_meta(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    key: &str,
) -> Result<Option<String>> {
    let project = scope.project_scope();
    client.query_opt(
        "SELECT value FROM scheduler_meta
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND key = $4",
        &[
            &project.platform_id().as_ref(),
            &project.company_id().as_ref(),
            &project.project_id().as_ref(),
            &key,
        ],
    )
    .context("load scoped PostgreSQL agent scheduler metadata")?
    .map(|row| row.try_get(0).context("read PostgreSQL scheduler metadata value"))
    .transpose()
}

fn upsert_meta(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    key: &str,
    value: &str,
) -> Result<()> {
    let project = scope.project_scope();
    client.execute(
        "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (platform_id, company_id, project_id, key)
         DO UPDATE SET value = excluded.value",
        &[
            &project.platform_id().as_ref(),
            &project.company_id().as_ref(),
            &project.project_id().as_ref(),
            &key,
            &value,
        ],
    ).context("save scoped PostgreSQL agent scheduler metadata")?;
    Ok(())
}

pub(super) fn load_task_with_client(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_id: &TaskId,
) -> Result<Option<StoredAgentTask>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {TASK_COLUMNS} FROM scheduler_tasks
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &task_id.as_str(),
            ],
        )
        .with_context(|| format!("load scoped PostgreSQL agent task: {}", task_id.as_str()))?
        .as_ref()
        .map(map_task_row)
        .transpose()
}

pub(super) fn load_task_for_update(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    task_id: &TaskId,
) -> Result<Option<StoredAgentTask>> {
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {TASK_COLUMNS} FROM scheduler_tasks
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4
                 FOR UPDATE"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &task_id.as_str(),
            ],
        )
        .with_context(|| format!("lock scoped PostgreSQL agent task: {}", task_id.as_str()))?
        .as_ref()
        .map(map_task_row)
        .transpose()
}

fn map_task_row(row: &Row) -> Result<StoredAgentTask> {
    let id_value: String = row.try_get(0).context("read PostgreSQL task id")?;
    let task_id = TaskId::new(id_value).context("parse PostgreSQL task id")?;
    let kind_key: String = row.try_get(1).context("read PostgreSQL task kind")?;
    let status_key: String = row.try_get(2).context("read PostgreSQL task status")?;
    let assigned_to: Option<String> = row.try_get(6).context("read PostgreSQL task worker")?;
    let team_id: Option<String> = row.try_get(7).context("read PostgreSQL task team")?;
    let delegated_to_team: Option<String> =
        row.try_get(10).context("read PostgreSQL delegated team")?;
    let dag_json: String = row.try_get(15).context("read PostgreSQL task DAG")?;
    let metadata_json: String = row.try_get(16).context("read PostgreSQL task metadata")?;
    let dag = parse_dag_value(&dag_json)
        .with_context(|| format!("parse PostgreSQL task DAG: {}", task_id.as_str()))?;
    let metadata = serde_json::from_str(&metadata_json)
        .with_context(|| format!("parse PostgreSQL task metadata: {}", task_id.as_str()))?;
    Ok(StoredAgentTask {
        id: task_id,
        kind: AgentTaskKind::parse(&kind_key).context("parse PostgreSQL task kind")?,
        status: AgentTaskStatus::parse(&status_key).context("parse PostgreSQL task status")?,
        title: row.try_get(3)?,
        spec_uri: row.try_get(4)?,
        meta_uri: row.try_get(5)?,
        assigned_to: assigned_to
            .map(AssigneeId::new)
            .transpose()
            .context("parse PostgreSQL task worker")?,
        team_id: team_id.map(TeamId::new).transpose().context("parse PostgreSQL task team")?,
        room_id: row.try_get(8)?,
        project_room_id: row.try_get(9)?,
        delegated_to_team: delegated_to_team
            .map(TeamId::new)
            .transpose()
            .context("parse PostgreSQL delegated team")?,
        schedule: row.try_get(11)?,
        timezone: row.try_get(12)?,
        next_scheduled_at: row.try_get(13)?,
        last_executed_at: row.try_get(14)?,
        dag,
        metadata,
        created_at_ms: TimestampMs(row.try_get(17)?),
        updated_at_ms: TimestampMs(row.try_get(18)?),
    })
}

fn push_filter(
    query: &mut String,
    values: &mut Vec<Box<dyn ToSql + Sync>>,
    column: &str,
    value: String,
) {
    values.push(Box::new(value));
    query.push_str(&format!(" AND {column} = ${}", values.len()));
}

fn ensure_single_update(changed: u64, task_id: &TaskId, operation: &str) -> Result<()> {
    if changed != 1 {
        bail!("PostgreSQL task {operation} affected {changed} rows: {}", task_id.as_str());
    }
    Ok(())
}

#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tasks_tests;
