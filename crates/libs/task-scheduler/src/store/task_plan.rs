//! Sparse, project-scoped task placement and shared validation for both stores.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use vw_api_agent_scheduler::{AgentSchedulerScope, AgentSchedulerTaskPropertyChanges};
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use super::events::AppendAgentEventRequest;
use super::tasks::{
    AgentTaskBoardUpdate, NewAgentTask, StoredAgentTask, bool_metadata_field, load_task_with_conn,
    load_workflow_with_conn,
};
use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    postgres_provider, scope_params,
};
use crate::domain::TaskId;
use crate::events::AgentEventKind;
use crate::lifecycle_bridge::{LifecycleBridge, builtin_task_workflow_snapshot};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPlanPlacement {
    Root { outer_batch: u32, inner_batch: u32 },
    Child { parent_task_id: TaskId, batch: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPlanEntry {
    pub task_id: TaskId,
    pub placement: TaskPlanPlacement,
}

impl AgentSchedulerStore {
    pub(crate) fn load_plan_dispatch_snapshot(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<PlanDispatchSnapshot> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.load_plan_dispatch_snapshot(&scope);
        }
        let conn = open_initialized_connection(self, context)?;
        load_plan_dispatch_snapshot(&conn, &scope, None)
    }

    /// Read the sparse project snapshot in one query, keyed by each entry's task ID.
    pub fn list_task_plan(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Vec<TaskPlanEntry>> {
        self.list_task_plan_entries(context, None)
    }

    pub fn list_task_plan_children(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent_task_id: &TaskId,
    ) -> Result<Vec<TaskPlanEntry>> {
        self.list_task_plan_entries(context, Some(parent_task_id))
    }

    fn list_task_plan_entries(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent_task_id: Option<&TaskId>,
    ) -> Result<Vec<TaskPlanEntry>> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_task_plan(&scope, parent_task_id);
        }
        let conn = open_initialized_connection(self, context)?;
        let (platform, company, project) = scope_params(&scope);
        let mut stmt = conn.prepare(SELECT_PLAN)?;
        let rows = stmt.query_map(
            params![platform, company, project, parent_task_id.map(TaskId::as_str)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        rows.map(|row| {
            let (id, parent, outer, batch) = row?;
            decode_entry(id, parent, outer, batch)
        }).collect()
    }

    pub fn create_task_with_plan(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task: NewAgentTask,
        placement: TaskPlanPlacement,
    ) -> Result<StoredAgentTask> {
        self.create_task_with_placement(context, task, Some(placement))
    }

    /// Atomically edit a task and replace its placement. Passing None clears both root batches.
    pub fn edit_task_with_plan(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        placement: Option<TaskPlanPlacement>,
        property_changes: AgentSchedulerTaskPropertyChanges,
        edit: impl FnOnce(&StoredAgentTask) -> Result<AgentTaskBoardUpdate> + Send,
    ) -> Result<Option<(StoredAgentTask, StoredAgentTask)>> {
        self.edit_task_with_placement(context, task_id, property_changes, Some(placement), edit)
    }
}

pub(super) const SELECT_PLAN: &str =
    "SELECT task_id, parent_task_id, outer_batch, batch
     FROM scheduler_task_plans
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
       AND (CAST($4 AS TEXT) IS NULL OR parent_task_id = $4)
     ORDER BY CASE WHEN parent_task_id IS NULL THEN 0 ELSE 1 END,
              parent_task_id, outer_batch, batch, task_id";

pub(super) const SELECT_MEMBERS: &str =
    "SELECT task.id, plan.parent_task_id, plan.outer_batch, plan.batch, task.status,
            task.metadata_json, EXISTS (
                SELECT 1 FROM scheduler_runs AS run
                WHERE run.platform_id = task.platform_id AND run.company_id = task.company_id
                  AND run.project_id = task.project_id AND run.task_id = task.id
            ) OR EXISTS (
                SELECT 1 FROM scheduler_task_plan_publications AS publication
                WHERE publication.platform_id = task.platform_id AND publication.company_id = task.company_id
                  AND publication.project_id = task.project_id AND publication.parent_task_id = task.id
            )
     FROM scheduler_tasks AS task
     LEFT JOIN scheduler_task_plans AS plan
       ON plan.platform_id = task.platform_id AND plan.company_id = task.company_id
      AND plan.project_id = task.project_id AND plan.task_id = task.id
     WHERE task.platform_id = $1 AND task.company_id = $2 AND task.project_id = $3
       AND (task.id = $4 OR plan.task_id IS NOT NULL)
     ORDER BY task.id";

// Include unbatched parents. At claim time restrict the read to the target's root
// batches and, for a child, its own parent and siblings.
pub(super) const SELECT_DISPATCH_MEMBERS: &str =
    "WITH target AS (
         SELECT parent_task_id FROM scheduler_task_plans
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4
     )
     SELECT task.id, plan.parent_task_id, plan.outer_batch, plan.batch, task.status,
            task.metadata_json, publication.parent_task_id IS NOT NULL
     FROM scheduler_tasks AS task
     LEFT JOIN scheduler_task_plans AS plan
       ON plan.platform_id = task.platform_id AND plan.company_id = task.company_id
      AND plan.project_id = task.project_id AND plan.task_id = task.id
     LEFT JOIN scheduler_task_plan_publications AS publication
       ON publication.platform_id = task.platform_id AND publication.company_id = task.company_id
      AND publication.project_id = task.project_id AND publication.parent_task_id = task.id
     WHERE task.platform_id = $1 AND task.company_id = $2 AND task.project_id = $3
       AND (
           (CAST($4 AS TEXT) IS NULL AND (plan.task_id IS NOT NULL OR publication.parent_task_id IS NOT NULL))
           OR EXISTS (
               SELECT 1 FROM target
               WHERE plan.outer_batch IS NOT NULL
                  OR task.id = target.parent_task_id
                  OR plan.parent_task_id = target.parent_task_id
           )
       )
     ORDER BY task.id";

pub(super) fn load_plan_dispatch_snapshot(
    conn: &Connection,
    scope: &AgentSchedulerScope,
    task_id: Option<&TaskId>,
) -> Result<PlanDispatchSnapshot> {
    let (platform, company, project) = scope_params(scope);
    let mut stmt = conn.prepare(SELECT_DISPATCH_MEMBERS)?;
    let rows = stmt.query_map(
        params![platform, company, project, task_id.map(TaskId::as_str)],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    PlanDispatchSnapshot::from_rows(rows.map(|row| row.map_err(Into::into)))
}

pub(super) type DispatchRow = (String, Option<String>, Option<i64>, Option<i64>, String, String, bool);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPlanDispatchBlocker {
    WaitingForEarlierBatch { task_ids: Vec<TaskId> },
    EarlierBatchFailed { task_ids: Vec<TaskId> },
    ParentNotReleased { parent_task_id: TaskId },
}

pub(crate) struct PlanDispatchSnapshot {
    members: BTreeMap<String, (PlanMember, bool)>,
}

impl PlanDispatchSnapshot {
    pub(super) fn from_rows(rows: impl Iterator<Item = Result<DispatchRow>>) -> Result<Self> {
        let members = rows.map(|row| {
            let (id, parent, outer, batch, status, metadata, published) = row?;
            let member = PlanMember::decode(id, parent, outer, batch, status, metadata, false)?;
            Ok((member.id.as_str().to_owned(), (member, published)))
        }).collect::<Result<_>>()?;
        Ok(Self { members })
    }

    pub(crate) fn blocker_for(
        &self,
        task_id: &TaskId,
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Option<TaskPlanDispatchBlocker> {
        let placement = self.members.get(task_id.as_str())?.0.placement.as_ref()?;
        let bridge = LifecycleBridge::new(workflow);
        if let TaskPlanPlacement::Child { parent_task_id, .. } = placement {
            let Some((parent, true)) = self.members.get(parent_task_id.as_str()) else {
                return Some(TaskPlanDispatchBlocker::ParentNotReleased {
                    parent_task_id: parent_task_id.clone(),
                });
            };
            if let Some(blocker) = parent.placement.as_ref()
                .and_then(|parent_placement| self.earlier_batch_blocker(parent_placement, &bridge))
            {
                return Some(blocker);
            }
            // The publishing transaction parks the parent until its children finish.
            if parent.status != AgentTaskStatus::Blocked || hidden(parent.status, &parent.metadata)
            {
                return Some(TaskPlanDispatchBlocker::ParentNotReleased {
                    parent_task_id: parent_task_id.clone(),
                });
            }
        }
        self.earlier_batch_blocker(placement, &bridge)
    }

    fn earlier_batch_blocker(
        &self,
        placement: &TaskPlanPlacement,
        bridge: &LifecycleBridge<'_>,
    ) -> Option<TaskPlanDispatchBlocker> {
        let mut waiting = Vec::new();
        let mut failed = Vec::new();
        for (member, _) in self.members.values() {
            if !member.placement.as_ref().is_some_and(|earlier| earlier.precedes(placement))
                || member.done(bridge)
            {
                continue;
            }
            if matches!(member.status, AgentTaskStatus::Failed | AgentTaskStatus::Cancelled)
                || hidden(member.status, &member.metadata)
            {
                failed.push(member.id.clone());
            } else {
                waiting.push(member.id.clone());
            }
        }
        if !failed.is_empty() {
            Some(TaskPlanDispatchBlocker::EarlierBatchFailed { task_ids: failed })
        } else if !waiting.is_empty() {
            Some(TaskPlanDispatchBlocker::WaitingForEarlierBatch { task_ids: waiting })
        } else {
            None
        }
    }
}

pub(super) const UPSERT_PLAN: &str =
    "INSERT INTO scheduler_task_plans
        (platform_id, company_id, project_id, task_id, parent_task_id, outer_batch, batch)
     VALUES ($1, $2, $3, $4, $5, $6, $7)
     ON CONFLICT (platform_id, company_id, project_id, task_id) DO UPDATE
     SET parent_task_id = excluded.parent_task_id, outer_batch = excluded.outer_batch,
         batch = excluded.batch";

pub(super) const DELETE_PLAN: &str =
    "DELETE FROM scheduler_task_plans
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4";

impl TaskPlanPlacement {
    pub(super) fn columns(&self) -> (Option<&str>, Option<i64>, i64) {
        match self {
            Self::Root { outer_batch, inner_batch } =>
                (None, Some(i64::from(*outer_batch)), i64::from(*inner_batch)),
            Self::Child { parent_task_id, batch } =>
                (Some(parent_task_id.as_str()), None, i64::from(*batch)),
        }
    }

    fn precedes(&self, other: &Self) -> bool {
        let (parent, outer, batch) = self.columns();
        let (other_parent, other_outer, other_batch) = other.columns();
        parent == other_parent && (outer, batch) < (other_outer, other_batch)
    }
}

pub(super) fn decode_entry(
    id: String,
    parent: Option<String>,
    outer: Option<i64>,
    batch: i64,
) -> Result<TaskPlanEntry> {
    let batch = u32::try_from(batch)?;
    let placement = match (parent, outer) {
        (None, Some(outer)) if outer > 0 && batch > 0 =>
            TaskPlanPlacement::Root { outer_batch: u32::try_from(outer)?, inner_batch: batch },
        (Some(parent), None) if batch > 0 =>
            TaskPlanPlacement::Child { parent_task_id: TaskId::new(parent)?, batch },
        _ => bail!("invalid task plan row: {id}"),
    };
    Ok(TaskPlanEntry { task_id: TaskId::new(id)?, placement })
}

pub(super) struct PlanMember {
    id: TaskId,
    placement: Option<TaskPlanPlacement>,
    status: AgentTaskStatus,
    metadata: Value,
    execution_started: bool,
}

impl PlanMember {
    pub(super) fn decode(
        id: String,
        parent: Option<String>,
        outer: Option<i64>,
        batch: Option<i64>,
        status: String,
        metadata: String,
        execution_started: bool,
    ) -> Result<Self> {
        Ok(Self {
            placement: batch.map(|batch| decode_entry(id.clone(), parent, outer, batch))
                .transpose()?.map(|entry| entry.placement),
            id: TaskId::new(id)?,
            status: AgentTaskStatus::parse(&status)?,
            metadata: serde_json::from_str(&metadata)?,
            execution_started,
        })
    }

    fn started(&self) -> bool {
        self.execution_started || !matches!(
            self.status,
            AgentTaskStatus::Draft | AgentTaskStatus::Queued
                | AgentTaskStatus::Paused | AgentTaskStatus::Blocked
        )
    }

    fn done(&self, bridge: &LifecycleBridge<'_>) -> bool {
        !hidden(self.status, &self.metadata)
            && self.status == AgentTaskStatus::Completed
            && bridge.status(&crate::task_board::task_board_status_key_for(self.status, &self.metadata))
                .is_ok_and(|status| status.role == TaskPoolWorkflowStatusRoleDto::Done)
    }

    fn is_child_of(&self, id: &TaskId) -> bool {
        matches!(&self.placement, Some(TaskPlanPlacement::Child { parent_task_id, .. }) if parent_task_id == id)
    }
}

pub(crate) fn hidden(status: AgentTaskStatus, metadata: &Value) -> bool {
    status == AgentTaskStatus::Archived
        || bool_metadata_field(metadata, "task_board.archived")
        || bool_metadata_field(metadata, "task_board.deleted")
}

pub(super) fn became_hidden(
    current: &StoredAgentTask, status: AgentTaskStatus, metadata: &Value,
) -> bool {
    // Treat soft deletion separately so an already archived task cannot bypass the guard.
    (!hidden(current.status, &current.metadata) && hidden(status, metadata))
        || (!bool_metadata_field(&current.metadata, "task_board.deleted")
            && bool_metadata_field(metadata, "task_board.deleted"))
}

pub(super) fn validate_hiding(members: &[PlanMember], task_id: &TaskId) -> Result<()> {
    let placement = members.iter().find(|member| &member.id == task_id)
        .and_then(|member| member.placement.as_ref());
    // A child is also a prerequisite of its parent's completion, even in the last batch.
    if let Some(TaskPlanPlacement::Child { parent_task_id, .. }) = placement {
        bail!("cannot hide task {} because of parent {}", task_id.as_str(), parent_task_id.as_str());
    }
    for member in members {
        if member.is_child_of(task_id)
            || placement.zip(member.placement.as_ref()).is_some_and(|(old, other)| old.precedes(other))
        {
            bail!("cannot hide task {} because of task {}", task_id.as_str(), member.id.as_str());
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlacementWrite {
    Create,
    RetryCreate,
    Edit,
}

pub(super) fn validate_placement(
    members: &[PlanMember],
    workflow: Option<TaskPoolWorkflowSnapshotDto>,
    task: &StoredAgentTask,
    replacement: Option<&TaskPlanPlacement>,
    parent: Option<&StoredAgentTask>,
    mode: PlacementWrite,
) -> Result<bool> {
    let target = members.iter().find(|member| member.id == task.id)
        .context("missing task in plan validation")?;
    let previous = target.placement.as_ref();
    if replacement == previous {
        return Ok(false);
    }
    let id = task.id.as_str();
    if mode == PlacementWrite::RetryCreate {
        bail!("task {id} already exists with a different placement");
    }
    if task.kind != AgentTaskKind::Finite || hidden(task.status, &task.metadata) || target.started() {
        bail!("cannot change placement of task {id}");
    }
    if let Some(replacement) = replacement {
        let (_, outer, batch) = replacement.columns();
        if outer == Some(0) || batch == 0 {
            bail!("task {id} batches must be positive");
        }
    }
    match (previous, replacement) {
        (Some(TaskPlanPlacement::Child { parent_task_id, .. }),
         Some(TaskPlanPlacement::Child { parent_task_id: next_parent, .. }))
            if parent_task_id == next_parent => {}
        (Some(TaskPlanPlacement::Child { .. }), _) =>
            bail!("cannot change the parent of task {id}"),
        (_, Some(TaskPlanPlacement::Child { parent_task_id, .. })) => {
            let parent_id = parent_task_id.as_str();
            if mode != PlacementWrite::Create || parent_task_id == &task.id
                || parent.is_none_or(|parent| parent.kind != AgentTaskKind::Finite
                    || hidden(parent.status, &parent.metadata))
                || members.iter().any(|member| &member.id == parent_task_id
                    && matches!(member.placement, Some(TaskPlanPlacement::Child { .. })))
            {
                bail!("invalid parent {parent_id} for task {id}");
            }
        }
        _ => {}
    }

    let workflow = workflow.unwrap_or_else(builtin_task_workflow_snapshot);
    let bridge = LifecycleBridge::new(&workflow);
    let incomplete = !target.done(&bridge);
    for member in members {
        let Some(other) = member.placement.as_ref() else { continue };
        if member.id == task.id {
            continue;
        }
        let was_before = previous.is_some_and(|old| old.precedes(other));
        let will_be_before = replacement.is_some_and(|new| new.precedes(other));
        if incomplete && !was_before && will_be_before && member.started() {
            bail!("task {id} placement conflicts with task {}", member.id.as_str());
        }
    }
    Ok(true)
}

pub(super) fn write_placement(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    scope: &AgentSchedulerScope,
    task: &StoredAgentTask,
    replacement: Option<&TaskPlanPlacement>,
    mode: PlacementWrite,
) -> Result<bool> {
    let members = load_plan_members(conn, scope, &task.id)?;
    let parent = match replacement {
        Some(TaskPlanPlacement::Child { parent_task_id, .. }) =>
            load_task_with_conn(conn, context, parent_task_id)?,
        _ => None,
    };
    if !validate_placement(
        &members, load_workflow_with_conn(conn, scope)?, task, replacement, parent.as_ref(), mode,
    )? {
        return Ok(false);
    }
    let (platform, company, project) = scope_params(scope);
    if let Some(placement) = replacement {
        let (parent, outer, batch) = placement.columns();
        conn.execute(UPSERT_PLAN, params![platform, company, project, task.id.as_str(), parent, outer, batch])?;
    } else {
        conn.execute(DELETE_PLAN, params![platform, company, project, task.id.as_str()])?;
    }
    Ok(true)
}

pub(super) fn load_plan_members(
    conn: &Connection, scope: &AgentSchedulerScope, task_id: &TaskId,
) -> Result<Vec<PlanMember>> {
    let (platform, company, project) = scope_params(scope);
    let mut stmt = conn.prepare(SELECT_MEMBERS)?;
    let rows = stmt.query_map(params![platform, company, project, task_id.as_str()], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
    })?;
    rows.map(|row| {
        let (id, parent, outer, batch, status, metadata, has_run) = row?;
        PlanMember::decode(id, parent, outer, batch, status, metadata, has_run)
    }).collect()
}

pub(super) fn placement_event(
    task: &StoredAgentTask, placement: Option<&TaskPlanPlacement>, creating: bool,
) -> AppendAgentEventRequest {
    let payload = placement.map(|placement| {
        let (parent_task_id, outer_batch, batch) = placement.columns();
        json!({ "parent_task_id": parent_task_id, "outer_batch": outer_batch, "batch": batch })
    });
    AppendAgentEventRequest {
        task_id: Some(task.id.clone()),
        run_id: None,
        worker_id: None,
        team_id: None,
        kind: if creating { AgentEventKind::TaskCreated } else { AgentEventKind::HumanIntervened },
        from_status: None,
        to_status: Some(task.status),
        from_status_key: None,
        to_status_key: Some(crate::task_board::task_board_status_key(task)),
        message: "task plan placement changed".to_string(),
        payload: json!({ "task_plan": payload }),
        created_at_ms: task.updated_at_ms,
    }
}
