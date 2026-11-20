//! Sparse, project-scoped task placement and shared validation for both stores.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use vw_api_agent_scheduler::{AgentSchedulerScope, AgentSchedulerTaskPropertyChanges, TaskPlanSubTask, WorkPlanPublication};
use vw_api_transport::common::TimestampMs;
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use super::events::{AppendAgentEventRequest, insert_event_with_conn};
use super::tasks::{
    AgentTaskBoardUpdate, NewAgentTask, StoredAgentTask, bool_metadata_field, load_task_with_conn,
    load_workflow_with_conn,
};
use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_initialized_connection,
    postgres_provider, scope_params,
};
use crate::domain::{RunId, TaskId};
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

#[derive(Debug, thiserror::Error)]
pub enum TaskPlanError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
}

pub(super) enum TaskPlanEdit {
    Preserve,
    Metadata,
    Replace(Option<TaskPlanPlacement>),
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
        self.list_task_plan_entries(context, None, None)
    }

    pub fn list_task_plan_children(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent_task_id: &TaskId,
    ) -> Result<Vec<TaskPlanEntry>> {
        self.list_task_plan_entries(context, Some(parent_task_id), None)
    }

    pub fn load_task_placement(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
    ) -> Result<Option<TaskPlanPlacement>> {
        Ok(self.list_task_plan_entries(context, None, Some(task_id))?
            .into_iter().next().map(|entry| entry.placement))
    }

    fn list_task_plan_entries(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent_task_id: Option<&TaskId>,
        task_id: Option<&TaskId>,
    ) -> Result<Vec<TaskPlanEntry>> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.list_task_plan(&scope, parent_task_id, task_id);
        }
        let conn = open_initialized_connection(self, context)?;
        let (platform, company, project) = scope_params(&scope);
        let mut stmt = conn.prepare(SELECT_PLAN)?;
        let rows = stmt.query_map(
            params![platform, company, project, parent_task_id.map(TaskId::as_str), task_id.map(TaskId::as_str)],
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
        self.edit_task_with_placement(context, task_id, property_changes, TaskPlanEdit::Replace(placement), edit)
    }
}

pub(super) const SELECT_PLAN: &str =
    "SELECT task_id, parent_task_id, outer_batch, batch
     FROM scheduler_task_plans
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
       AND (CAST($4 AS TEXT) IS NULL OR parent_task_id = $4)
       AND (CAST($5 AS TEXT) IS NULL OR task_id = $5)
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
            task.metadata_json, publication.planning_attempt_id, summary.value,
            EXISTS (
                SELECT 1 FROM scheduler_runs AS run
                WHERE run.platform_id = task.platform_id AND run.company_id = task.company_id
                  AND run.project_id = task.project_id AND run.task_id = task.id
                  AND run.finished_at_ms IS NULL AND run.status IN ('leased', 'running')
            )
     FROM scheduler_tasks AS task
     LEFT JOIN scheduler_task_plans AS plan
       ON plan.platform_id = task.platform_id AND plan.company_id = task.company_id
      AND plan.project_id = task.project_id AND plan.task_id = task.id
     LEFT JOIN scheduler_task_plan_publications AS publication
       ON publication.platform_id = task.platform_id AND publication.company_id = task.company_id
      AND publication.project_id = task.project_id AND publication.parent_task_id = task.id
     LEFT JOIN scheduler_meta AS summary
       ON summary.platform_id = task.platform_id AND summary.company_id = task.company_id
      AND summary.project_id = task.project_id AND summary.key = 'task_plan.summary:' || task.id
     WHERE task.platform_id = $1 AND task.company_id = $2 AND task.project_id = $3
       AND (
           (CAST($4 AS TEXT) IS NULL AND (plan.task_id IS NOT NULL OR publication.parent_task_id IS NOT NULL
               OR task.metadata_json LIKE '%\"subtasks\"%'))
           OR task.id = $4 OR plan.parent_task_id = $4
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
                row.get(7)?,
                row.get(8)?,
            ))
        },
    )?;
    PlanDispatchSnapshot::from_rows(rows.map(|row| row.map_err(Into::into)))
}

pub(super) type DispatchRow = (
    String, Option<String>, Option<i64>, Option<i64>, String, String,
    Option<String>, Option<String>, bool,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPlanDispatchBlocker {
    WaitingForEarlierBatch { task_ids: Vec<TaskId> },
    EarlierBatchFailed { task_ids: Vec<TaskId> },
    ParentNotReleased { parent_task_id: TaskId },
    WorkPlanNotPublished,
    LegacyWorkPlanRequiresResplit,
    WorkPlanChildrenIncomplete { children: Vec<(TaskId, String, String)> },
    WorkPlanSummaryUnavailable,
}

#[derive(PartialEq, Eq)]
enum SummaryState {
    Waiting,
    Queued,
    Claimed,
}

struct DispatchMember {
    task: PlanMember,
    publication_attempt: Option<String>,
    summary: SummaryState,
    active_run: bool,
}

pub(crate) struct PlanDispatchSnapshot {
    members: BTreeMap<String, DispatchMember>,
    children: BTreeMap<String, Vec<String>>,
}

impl PlanDispatchSnapshot {
    pub(super) fn from_rows(rows: impl Iterator<Item = Result<DispatchRow>>) -> Result<Self> {
        let mut children = BTreeMap::<String, Vec<String>>::new();
        let members = rows.map(|row| {
            let (id, parent, outer, batch, status, metadata, publication_attempt, summary, active_run) = row?;
            if let Some(parent_id) = &parent {
                children.entry(parent_id.clone()).or_default().push(id.clone());
            }
            let task = PlanMember::decode(id, parent, outer, batch, status, metadata, false)?;
            let summary = match summary.as_deref() {
                None => SummaryState::Waiting,
                Some(value) if publication_attempt.as_ref().is_some_and(|attempt| value == format!("pending:{attempt}")) =>
                    SummaryState::Queued,
                Some(_) => SummaryState::Claimed,
            };
            Ok((task.id.as_str().to_owned(), DispatchMember { task, publication_attempt, summary, active_run }))
        }).collect::<Result<_>>()?;
        Ok(Self { members, children })
    }

    pub(crate) fn blocker_for(
        &self,
        task_id: &TaskId,
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Option<TaskPlanDispatchBlocker> {
        let member = self.members.get(task_id.as_str());
        if member.is_some_and(|member| vw_api_agent_scheduler::has_legacy_work_plan(&member.task.metadata)) {
            return Some(TaskPlanDispatchBlocker::LegacyWorkPlanRequiresResplit);
        }
        if self.children.contains_key(task_id.as_str()) && !member.is_some_and(|member| member.publication_attempt.is_some()) {
            return Some(TaskPlanDispatchBlocker::WorkPlanNotPublished);
        }
        let member = member?;
        let bridge = LifecycleBridge::new(workflow);
        if member.publication_attempt.is_some() {
            if member.summary == SummaryState::Claimed {
                return Some(TaskPlanDispatchBlocker::WorkPlanSummaryUnavailable);
            }
            let children = self.incomplete_children(task_id, &bridge);
            if !children.is_empty() {
                return Some(TaskPlanDispatchBlocker::WorkPlanChildrenIncomplete { children });
            }
            if member.summary != SummaryState::Queued
                || member.task.status != AgentTaskStatus::Queued
                || !self.children.contains_key(task_id.as_str())
            {
                return Some(TaskPlanDispatchBlocker::WorkPlanSummaryUnavailable);
            }
        }
        let placement = member.task.placement.as_ref()?;
        if let TaskPlanPlacement::Child { parent_task_id, .. } = placement {
            let blocker = TaskPlanDispatchBlocker::ParentNotReleased {
                parent_task_id: parent_task_id.clone(),
            };
            let Some(parent) = self.members.get(parent_task_id.as_str()) else {
                return Some(blocker);
            };
            if parent.publication_attempt.is_none()
                || parent.summary == SummaryState::Claimed
                || !parent.task.waits_for_children(&bridge)
            {
                return Some(blocker);
            }
            if let Some(blocker) = parent.task.placement.as_ref()
                .and_then(|parent_placement| self.earlier_batch_blocker(parent_placement, &bridge))
            {
                return Some(blocker);
            }
        }
        self.earlier_batch_blocker(placement, &bridge)
    }

    fn incomplete_children(
        &self,
        parent: &TaskId,
        bridge: &LifecycleBridge<'_>,
    ) -> Vec<(TaskId, String, String)> {
        self.children.get(parent.as_str()).into_iter().flatten()
            .map(|id| &self.members[id])
            .filter(|child| child.active_run || !child.task.done(bridge))
            .map(|child| (
                child.task.id.clone(),
                child.task.status.as_str().to_owned(),
                crate::task_board::task_board_status_key_for(child.task.status, &child.task.metadata),
            ))
            .collect()
    }

    pub(super) fn summaries_to_queue(
        &self,
        workflow: &TaskPoolWorkflowSnapshotDto,
    ) -> Vec<(&TaskId, &str)> {
        let bridge = LifecycleBridge::new(workflow);
        self.members.values().filter_map(|member| {
            let attempt = member.publication_attempt.as_deref()?;
            let task = &member.task;
            if member.summary != SummaryState::Waiting
                || !task.waits_for_children(&bridge)
                || member.active_run
                || !self.children.contains_key(task.id.as_str())
                || !self.incomplete_children(&task.id, &bridge).is_empty()
            {
                return None;
            }
            Some((&task.id, attempt))
        }).collect()
    }

    fn earlier_batch_blocker(
        &self,
        placement: &TaskPlanPlacement,
        bridge: &LifecycleBridge<'_>,
    ) -> Option<TaskPlanDispatchBlocker> {
        let mut waiting = Vec::new();
        let mut failed = Vec::new();
        for member in self.members.values().map(|member| &member.task) {
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
    pub(super) fn id(&self) -> &TaskId {
        &self.id
    }

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

    fn waits_for_children(&self, bridge: &LifecycleBridge<'_>) -> bool {
        if hidden(self.status, &self.metadata) {
            return false;
        }
        let status_key = crate::task_board::task_board_status_key_for(self.status, &self.metadata);
        match self.status {
            AgentTaskStatus::Blocked => status_key == "blocked",
            AgentTaskStatus::Queued => bridge.dispatch_eligibility(&status_key)
                .is_ok_and(|eligibility| eligibility.eligible),
            _ => false,
        }
    }

    pub(super) fn is_child_of(&self, id: &TaskId) -> bool {
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

pub(super) fn validate_metadata_completion(
    members: &[PlanMember],
    task_id: &TaskId,
) -> Result<()> {
    if members.iter().any(|member| {
        member.is_child_of(task_id) || (&member.id == task_id && member.placement.is_some())
    }) {
        return Err(TaskPlanError::Conflict(format!(
            "cannot complete work plan task {} through metadata", task_id.as_str(),
        )).into());
    }
    Ok(())
}

pub(super) fn validate_hiding(
    members: &[PlanMember], task_id: &TaskId, archived_published_parent: bool,
) -> Result<()> {
    let placement = members.iter().find(|member| &member.id == task_id)
        .and_then(|member| member.placement.as_ref());
    // A child is also a prerequisite of its parent's completion, even in the last batch.
    if let Some(TaskPlanPlacement::Child { parent_task_id, .. }) = placement {
        return Err(TaskPlanError::Conflict(format!(
            "cannot hide task {} because of parent {}", task_id.as_str(), parent_task_id.as_str(),
        )).into());
    }
    for member in members {
        if (member.is_child_of(task_id) && !archived_published_parent)
            || placement.zip(member.placement.as_ref()).is_some_and(|(old, other)| old.precedes(other))
        {
            return Err(TaskPlanError::Conflict(format!(
                "cannot hide task {} because of task {}", task_id.as_str(), member.id.as_str(),
            )).into());
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
        return Err(TaskPlanError::Conflict(format!(
            "task {id} already exists with a different placement"
        )).into());
    }
    if task.kind != AgentTaskKind::Finite || hidden(task.status, &task.metadata) || target.started() {
        return Err(TaskPlanError::Conflict(format!("cannot change placement of task {id}")).into());
    }
    if let Some(replacement) = replacement {
        let (_, outer, batch) = replacement.columns();
        if outer == Some(0) || batch == 0 {
            return Err(TaskPlanError::Invalid(format!("task {id} batches must be positive")).into());
        }
    }
    match (previous, replacement) {
        (Some(TaskPlanPlacement::Child { parent_task_id, .. }),
         Some(TaskPlanPlacement::Child { parent_task_id: next_parent, .. }))
            if parent_task_id == next_parent => {}
        (Some(TaskPlanPlacement::Child { .. }), _) =>
            return Err(TaskPlanError::Conflict(format!("cannot change the parent of task {id}")).into()),
        (_, Some(TaskPlanPlacement::Child { parent_task_id, .. })) => {
            let parent_id = parent_task_id.as_str();
            if mode != PlacementWrite::Create {
                return Err(TaskPlanError::Conflict(format!(
                    "cannot assign a parent to existing task {id}"
                )).into());
            }
            let parent = parent.filter(|parent| !hidden(parent.status, &parent.metadata))
                .ok_or_else(|| TaskPlanError::NotFound(format!("parent task {parent_id} not found")))?;
            if parent.id == task.id || parent.kind != AgentTaskKind::Finite
                || members.iter().any(|member| &member.id == parent_task_id
                    && matches!(member.placement, Some(TaskPlanPlacement::Child { .. })))
            {
                return Err(TaskPlanError::Invalid(format!(
                    "invalid parent {parent_id} for task {id}"
                )).into());
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
            return Err(TaskPlanError::Conflict(format!(
                "task {id} placement conflicts with task {}", member.id.as_str(),
            )).into());
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
    if let Some(TaskPlanPlacement::Child { parent_task_id, .. }) = replacement
        && sqlite_publication_exists(conn, scope, parent_task_id)?
    {
        return Err(conflict("published child placements are frozen"));
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
    let plan = placement.map(|placement| {
        let (parent_task_id, outer_batch, batch) = placement.columns();
        json!({ "parent_task_id": parent_task_id, "outer_batch": outer_batch, "batch": batch })
    });
    let payload = if creating {
        json!({
            "task_plan": plan,
            "task_id": task.id.as_str(),
            "title": task.title,
            "kind": "finite",
            "status": task.status.as_str(),
        })
    } else {
        json!({ "task_plan": plan })
    };
    AppendAgentEventRequest {
        task_id: Some(task.id.clone()),
        run_id: None,
        worker_id: task.assigned_to.clone(),
        team_id: task.team_id.clone(),
        kind: if creating { AgentEventKind::TaskCreated } else { AgentEventKind::HumanIntervened },
        from_status: None,
        to_status: Some(task.status),
        from_status_key: None,
        to_status_key: Some(crate::task_board::task_board_status_key(task)),
        message: "task plan placement changed".to_string(),
        payload,
        created_at_ms: task.updated_at_ms,
    }
}

// The receipt lives in scoped scheduler metadata so it survives parent edits and
// can be read before any eligibility check on a replay.
pub(super) const SELECT_PUBLICATION: &str =
    "SELECT planning_attempt_id FROM scheduler_task_plan_publications
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND parent_task_id = $4";
pub(super) const INSERT_PUBLICATION: &str =
    "INSERT INTO scheduler_task_plan_publications
        (platform_id, company_id, project_id, parent_task_id, planning_attempt_id)
     VALUES ($1, $2, $3, $4, $5)";
pub(super) const SELECT_RECEIPT: &str =
    "SELECT value FROM scheduler_meta
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND key = $4";
pub(super) const INSERT_RECEIPT: &str =
    "INSERT INTO scheduler_meta (platform_id, company_id, project_id, key, value)
     VALUES ($1, $2, $3, $4, $5)";
pub(super) const INSERT_PLAN_CHILD: &str =
    "INSERT INTO scheduler_tasks (
         platform_id, company_id, project_id, id, title, metadata_json, created_at_ms,
         kind, status, updated_at_ms, assigned_to, team_id, room_id, project_room_id, delegated_to_team,
         spec_uri, meta_uri
     ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'finite', 'queued', $7, $8, $9, $10, $11, $12, $13, $14)";
pub(super) const UPDATE_PLAN_TASK: &str =
    "UPDATE scheduler_tasks SET status = $5, metadata_json = $6, updated_at_ms = $7
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND id = $4";
pub(super) const SELECT_PLANNING_RUNS: &str =
    "SELECT id FROM scheduler_runs
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4
       AND finished_at_ms IS NULL AND status IN ('leased', 'running')";
pub(super) const FINISH_PLANNING_RUN: &str =
    "UPDATE scheduler_runs SET status = 'succeeded', finished_at_ms = $6, lease_expires_at_ms = NULL
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND task_id = $4 AND id = $5
       AND finished_at_ms IS NULL AND status IN ('leased', 'running')";

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct PublicationReceipt {
    request_digest: String,
    child_ids: Vec<String>,
}

impl PublicationReceipt {
    pub(super) fn replay(value: Option<String>, request: &WorkPlanPublication) -> Result<Vec<String>> {
        let receipt: Self = serde_json::from_str(
            &value.ok_or_else(|| conflict("parent already has a published work plan"))?,
        )?;
        if receipt.request_digest != publication_digest(request)? {
            return Err(conflict("parent already has a different published work plan"));
        }
        Ok(receipt.child_ids)
    }

    pub(super) fn new(request: &WorkPlanPublication, child_ids: Vec<String>) -> Result<Self> {
        Ok(Self { request_digest: publication_digest(request)?, child_ids })
    }

    pub(super) fn into_child_ids(self) -> Vec<String> {
        self.child_ids
    }
}

fn publication_digest(request: &WorkPlanPublication) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(request)?)))
}

pub(super) fn publication_key(parent: &TaskId) -> String {
    format!("task_plan.publication:{}", parent.as_str())
}

pub(super) fn publication_attempt(request: &WorkPlanPublication) -> &str {
    match request {
        WorkPlanPublication::Generated { planning_attempt_id, .. } => planning_attempt_id,
        WorkPlanPublication::Manual => "manual",
    }
}

pub(super) fn conflict(message: &str) -> anyhow::Error {
    TaskPlanError::Conflict(message.to_owned()).into()
}

pub(super) fn missing_parent() -> TaskPlanError {
    TaskPlanError::NotFound("parent task not found".to_owned())
}

pub(super) fn validate_publication(
    parent: &StoredAgentTask,
    members: &[PlanMember],
    request: &WorkPlanPublication,
    active_runs: &[String],
) -> Result<()> {
    if hidden(parent.status, &parent.metadata) {
        return Err(missing_parent().into());
    }
    if parent.kind != AgentTaskKind::Finite || members.iter().any(|member| {
        member.id == parent.id && matches!(member.placement, Some(TaskPlanPlacement::Child { .. }))
    }) {
        return Err(conflict("only a finite root task can publish a work plan"));
    }
    let mut children = members.iter().filter(|member| member.is_child_of(&parent.id));
    match request {
        WorkPlanPublication::Generated { planning_attempt_id, subtasks } => {
            if parent.status != AgentTaskStatus::Running
                || active_runs.len() != 1
                || &active_runs[0] != planning_attempt_id
            {
                return Err(conflict("planning attempt no longer owns the parent run"));
            }
            if children.next().is_some() {
                return Err(conflict("parent already has manually placed children"));
            }
            validate_generated_tasks(subtasks)?;
        }
        WorkPlanPublication::Manual => {
            if !matches!(parent.status, AgentTaskStatus::Draft | AgentTaskStatus::Queued)
                || !active_runs.is_empty()
            {
                return Err(conflict("parent must be idle to start a manual work plan"));
            }
            let mut count = 0;
            for child in children {
                if child.started() || hidden(child.status, &child.metadata)
                    || !matches!(child.status, AgentTaskStatus::Draft | AgentTaskStatus::Queued)
                {
                    return Err(conflict("manual plan contains an ineligible child"));
                }
                count += 1;
            }
            if count == 0 {
                return Err(TaskPlanError::Invalid("work plan must contain children".into()).into());
            }
        }
    }
    Ok(())
}

fn validate_generated_tasks(subtasks: &[TaskPlanSubTask]) -> Result<()> {
    let invalid = |message: String| TaskPlanError::Invalid(message);
    if subtasks.is_empty() {
        return Err(invalid("work plan must contain children".into()).into());
    }
    let batches: BTreeSet<_> = subtasks.iter().map(|task| task.batch).collect();
    let mut files = BTreeMap::new();
    for (index, task) in subtasks.iter().enumerate() {
        if task.title.trim().is_empty() || task.execution_prompt.trim().is_empty() || task.batch == 0 {
            return Err(invalid(format!("invalid work plan child {}", index + 1)).into());
        }
        if task.depends_on_batches.iter().any(|batch| {
            *batch == 0 || *batch >= task.batch || !batches.contains(batch)
        }) {
            return Err(invalid(format!("invalid dependency batch for {}", task.title)).into());
        }
        for path in &task.target_files {
            let path = path.trim().replace('\\', "/");
            let mut parts = Vec::new();
            for part in path.split('/') {
                match part {
                    "" | "." => {}
                    ".." if parts.last().is_some_and(|last| *last != "..") => { parts.pop(); }
                    _ => parts.push(part),
                }
            }
            let normalized = parts.join("/");
            if !normalized.is_empty()
                && files.insert((task.batch, normalized), index).is_some_and(|other| other != index)
            {
                return Err(invalid(format!("conflicting target file in batch {}: {path}", task.batch)).into());
            }
        }
    }
    Ok(())
}

pub(super) fn queued_status_key(workflow: &TaskPoolWorkflowSnapshotDto) -> Result<&str> {
    workflow.statuses.iter()
        .find(|status| status.enabled && status.role == TaskPoolWorkflowStatusRoleDto::Queued)
        .or_else(|| workflow.statuses.iter().find(|status| {
            status.enabled && status.role == TaskPoolWorkflowStatusRoleDto::Planning
        }))
        .map(|status| status.key.as_str())
        .ok_or_else(|| TaskPlanError::Invalid("project workflow has no dispatchable status".into()).into())
}

pub(super) fn generated_child_id(parent_id: &TaskId, index: usize) -> Result<TaskId> {
    let parent = hex::encode(Sha256::digest(parent_id.as_str().as_bytes()));
    TaskId::new(format!("plan-{parent}-{}", index + 1))
}

pub(super) fn generated_child(
    parent: &StoredAgentTask,
    subtask: &TaskPlanSubTask,
    index: usize,
    workflow: &TaskPoolWorkflowSnapshotDto,
    queued_key: &str,
) -> Result<(TaskId, String)> {
    let id = generated_child_id(&parent.id, index)?;
    let mut metadata = serde_json::Map::new();
    for key in ["model", "agent", "acp_agent", "executor_id", "reasoning_effort", "priority"] {
        if let Some(value) = parent.metadata.get(key) {
            metadata.insert(key.into(), value.clone());
        }
    }
    metadata.insert("order_no".into(), json!(index));
    metadata.insert("prompt".into(), json!(subtask.execution_prompt));
    metadata.insert("task_split_mode".into(), json!("disabled"));
    metadata.insert("task_plan.context".into(), json!({
        "parent_task_id": parent.id.as_str(),
        "goal": parent.metadata.get("prompt").and_then(Value::as_str)
            .filter(|prompt| !prompt.trim().is_empty()).unwrap_or(&parent.title),
        "boundary": subtask.boundary,
        "acceptance_criteria": subtask.acceptance_criteria,
        "target_files": subtask.target_files,
        "depends_on_batches": subtask.depends_on_batches,
    }));
    crate::task_board::apply_workflow_status(workflow, queued_key, &mut metadata)?;
    Ok((id, serde_json::to_string(&metadata)?))
}

pub(super) fn manual_child_metadata(
    parent: &StoredAgentTask,
    child: &PlanMember,
    workflow: &TaskPoolWorkflowSnapshotDto,
    queued_key: &str,
) -> Result<String> {
    let mut metadata = child.metadata.as_object().cloned().unwrap_or_default();
    metadata.insert("task_plan.context".into(), json!({
        "parent_task_id": parent.id.as_str(),
        "goal": parent.metadata.get("prompt").and_then(Value::as_str)
            .filter(|prompt| !prompt.trim().is_empty()).unwrap_or(&parent.title),
    }));
    if child.status == AgentTaskStatus::Draft
        || !LifecycleBridge::new(workflow)
            .dispatch_eligibility(&crate::task_board::task_board_status_key_for(child.status, &child.metadata))
            .is_ok_and(|eligibility| eligibility.eligible)
    {
        crate::task_board::apply_workflow_status(workflow, queued_key, &mut metadata)?;
    }
    Ok(serde_json::to_string(&metadata)?)
}

pub(super) fn waiting_parent_metadata(parent: &StoredAgentTask) -> Result<String> {
    let mut metadata = parent.metadata.as_object().cloned().unwrap_or_default();
    metadata.insert("task_board.status_key".into(), json!("blocked"));
    metadata.remove("task_board.status_role");
    Ok(serde_json::to_string(&metadata)?)
}

pub(super) fn publication_event(
    parent: &StoredAgentTask,
    request: &WorkPlanPublication,
    children: &[String],
    now: TimestampMs,
) -> Result<AppendAgentEventRequest> {
    Ok(AppendAgentEventRequest {
        task_id: Some(parent.id.clone()),
        run_id: match request {
            WorkPlanPublication::Generated { planning_attempt_id, .. } =>
                Some(RunId::new(planning_attempt_id.clone())?),
            WorkPlanPublication::Manual => None,
        },
        worker_id: parent.assigned_to.clone(),
        team_id: parent.team_id.clone(),
        kind: AgentEventKind::TaskBlocked,
        from_status: Some(parent.status),
        to_status: Some(AgentTaskStatus::Blocked),
        from_status_key: Some(crate::task_board::task_board_status_key(parent)),
        to_status_key: Some("blocked".into()),
        message: "work plan published; waiting for children".into(),
        payload: json!({
            "planning_attempt_id": publication_attempt(request),
            "child_task_ids": children,
        }),
        created_at_ms: now,
    })
}

impl AgentSchedulerStore {
    /// Recover completion notifications from committed task and run state.
    pub fn queue_completed_work_plans(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now: TimestampMs,
    ) -> Result<usize> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.queue_completed_work_plans(&scope, now);
        }
        let mut conn = open_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let workflow = crate::engine::dispatch::effective_dispatch_workflow(
            load_workflow_with_conn(&tx, &scope)?,
        )?;
        let snapshot = load_plan_dispatch_snapshot(&tx, &scope, None)?;
        let (platform, company, project) = scope_params(&scope);
        let mut queued = 0;
        for (id, attempt) in snapshot.summaries_to_queue(&workflow) {
            let parent = load_task_with_conn(&tx, context, id)?.context("work plan parent disappeared")?;
            let Some((metadata, event)) = prepare_summary(&parent, &workflow, attempt, now)? else {
                continue;
            };
            tx.execute(INSERT_RECEIPT, params![
                platform, company, project, summary_key(id), format!("pending:{attempt}")
            ])?;
            tx.execute(&UPDATE_PLAN_TASK.replace('$', "?"), params![
                platform, company, project, id.as_str(), "queued", metadata, now.0
            ])?;
            insert_event_with_conn(&tx, &scope, event)?;
            queued += 1;
        }
        tx.commit()?;
        Ok(queued)
    }

    pub(crate) fn is_work_plan_summary_run(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent: &TaskId,
        run: &RunId,
    ) -> Result<bool> {
        let scope = context.storage_scope()?;
        let receipt = if let Some(store) = postgres_provider(self, context)? {
            store.summary_receipt(&scope, parent)?
        } else {
            let conn = open_initialized_connection(self, context)?;
            let (platform, company, project) = scope_params(&scope);
            conn.query_row(SELECT_RECEIPT, params![platform, company, project, summary_key(parent)], |row| {
                row.get::<_, String>(0)
            }).optional()?
        };
        Ok(receipt.as_deref().and_then(|value| value.strip_prefix("run:")) == Some(run.as_str()))
    }

    pub fn work_plan_is_published(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent: &TaskId,
    ) -> Result<bool> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.work_plan_is_published(&scope, parent);
        }
        let conn = open_initialized_connection(self, context)?;
        sqlite_publication_exists(&conn, &scope, parent)
    }

    pub fn publish_work_plan(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        parent_id: &TaskId,
        request: &WorkPlanPublication,
        now: TimestampMs,
    ) -> Result<Vec<String>> {
        let scope = context.storage_scope()?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.publish_work_plan(&scope, parent_id, request, now);
        }
        let (platform, company, project) = scope_params(&scope);
        let mut conn = open_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let key = publication_key(parent_id);
        if sqlite_publication_exists(&tx, &scope, parent_id)? {
            let receipt = tx.query_row(SELECT_RECEIPT, params![platform, company, project, key], |row| {
                row.get(0)
            }).optional()?;
            return PublicationReceipt::replay(receipt, request);
        }
        let parent = load_task_with_conn(&tx, context, parent_id)?.ok_or_else(missing_parent)?;
        let members = load_plan_members(&tx, &scope, parent_id)?;
        let active_runs = {
            let mut stmt = tx.prepare(SELECT_PLANNING_RUNS)?;
            stmt.query_map(params![platform, company, project, parent_id.as_str()], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<String>>>()?
        };
        validate_publication(&parent, &members, request, &active_runs)?;
        let workflow = crate::engine::dispatch::effective_dispatch_workflow(
            load_workflow_with_conn(&tx, &scope)?,
        )?;
        let queued_key = queued_status_key(&workflow)?;
        let update_task = UPDATE_PLAN_TASK.replace('$', "?");
        let mut child_ids = Vec::new();
        match request {
            WorkPlanPublication::Generated { planning_attempt_id, subtasks } => {
                for (index, subtask) in subtasks.iter().enumerate() {
                    let (id, metadata) = generated_child(&parent, subtask, index, &workflow, queued_key)?;
                    tx.execute(INSERT_PLAN_CHILD, params![
                        platform, company, project, id.as_str(), subtask.title, metadata, now.0,
                        parent.assigned_to.as_ref().map(|id| id.as_str()),
                        parent.team_id.as_ref().map(|id| id.as_str()),
                        parent.room_id, parent.project_room_id,
                        parent.delegated_to_team.as_ref().map(|id| id.as_str()),
                        format!("task-board/{}/spec.md", id.as_str()),
                        format!("task-board/{}/meta.json", id.as_str()),
                    ])?;
                    let placement = TaskPlanPlacement::Child { parent_task_id: parent_id.clone(), batch: subtask.batch };
                    tx.execute(UPSERT_PLAN, params![platform, company, project, id.as_str(),
                        parent_id.as_str(), Option::<i64>::None, i64::from(subtask.batch)])?;
                    let child = load_task_with_conn(&tx, context, &id)?.context("published child disappeared")?;
                    insert_event_with_conn(&tx, &scope, placement_event(&child, Some(&placement), true))?;
                    child_ids.push(id.as_str().to_owned());
                }
                if tx.execute(&FINISH_PLANNING_RUN.replace('$', "?"), params![
                    platform, company, project, parent_id.as_str(), planning_attempt_id, now.0
                ])? != 1 {
                    return Err(conflict("planning run ended before publication"));
                }
            }
            WorkPlanPublication::Manual => {
                for child in members.iter().filter(|member| member.is_child_of(parent_id)) {
                    let metadata = manual_child_metadata(&parent, child, &workflow, queued_key)?;
                    tx.execute(&update_task, params![
                        platform, company, project, child.id.as_str(), "queued", metadata, now.0
                    ])?;
                    child_ids.push(child.id.as_str().to_owned());
                }
            }
        }
        tx.execute(&update_task, params![
            platform, company, project, parent_id.as_str(), "blocked", waiting_parent_metadata(&parent)?, now.0
        ])?;
        tx.execute(INSERT_PUBLICATION, params![platform, company, project, parent_id.as_str(), publication_attempt(request)])?;
        insert_event_with_conn(&tx, &scope, publication_event(&parent, request, &child_ids, now)?)?;
        let receipt = PublicationReceipt::new(request, child_ids)?;
        tx.execute(INSERT_RECEIPT, params![platform, company, project, key, serde_json::to_string(&receipt)?])?;
        tx.commit()?;
        Ok(receipt.into_child_ids())
    }
}

pub(super) fn sqlite_publication_exists(conn: &Connection, scope: &AgentSchedulerScope, parent: &TaskId) -> Result<bool> {
    let (platform, company, project) = scope_params(scope);
    Ok(conn.query_row(SELECT_PUBLICATION, params![platform, company, project, parent.as_str()], |row| {
        row.get::<_, String>(0)
    }).optional()?.is_some())
}


pub(super) fn summary_key(parent: &TaskId) -> String {
    format!("task_plan.summary:{}", parent.as_str())
}

pub(super) const CLAIM_SUMMARY: &str =
    "UPDATE scheduler_meta SET value = $5
     WHERE platform_id = $1 AND company_id = $2 AND project_id = $3 AND key = $4
       AND value LIKE 'pending:%'";

pub(super) fn claim_summary(
    conn: &Connection, scope: &AgentSchedulerScope, task: &TaskId, run: &RunId,
) -> Result<()> {
    let (platform, company, project) = scope_params(scope);
    conn.execute(&CLAIM_SUMMARY.replace('$', "?"), params![
        platform, company, project, summary_key(task), format!("run:{}", run.as_str())
    ])?;
    Ok(())
}

// Relationship and summary lease are project-scoped. Check in the same writer
// transaction as status updates, so reopening and claiming cannot race.
pub(super) const SELECT_CLAIMED_PARENT: &str =
    "SELECT plan.parent_task_id FROM scheduler_task_plans AS plan
     JOIN scheduler_meta AS summary
       ON summary.platform_id = plan.platform_id AND summary.company_id = plan.company_id
      AND summary.project_id = plan.project_id AND summary.key = 'task_plan.summary:' || plan.parent_task_id
     WHERE plan.platform_id = $1 AND plan.company_id = $2 AND plan.project_id = $3
       AND plan.task_id = $4 AND summary.value LIKE 'run:%'";

pub(super) fn changes_completion(
    current: &StoredAgentTask, status: AgentTaskStatus, metadata: &Value,
) -> bool {
    current.status == AgentTaskStatus::Completed
        && (status != current.status || hidden(status, metadata)
            || crate::task_board::task_board_status_key(current)
                != crate::task_board::task_board_status_key_for(status, metadata))
}

pub(super) fn validate_summary_child_update(
    conn: &Connection, scope: &AgentSchedulerScope, current: &StoredAgentTask,
    status: AgentTaskStatus, metadata: &Value,
) -> Result<()> {
    if changes_completion(current, status, metadata) {
        let (platform, company, project) = scope_params(scope);
        let parent = conn.query_row(
            SELECT_CLAIMED_PARENT, params![platform, company, project, current.id.as_str()],
            |row| row.get::<_, String>(0),
        ).optional()?;
        reject_claimed_child_update(parent)?;
    }
    Ok(())
}

pub(super) fn reject_claimed_child_update(parent: Option<String>) -> Result<()> {
    if let Some(parent) = parent {
        return Err(conflict(&format!("parent {parent} already claimed its work plan summary")));
    }
    Ok(())
}

pub(super) fn prepare_summary(
    parent: &StoredAgentTask, workflow: &TaskPoolWorkflowSnapshotDto, attempt: &str, now: TimestampMs,
) -> Result<Option<(String, AppendAgentEventRequest)>> {
    let bridge = LifecycleBridge::new(workflow);
    let Ok(queued_key) = queued_status_key(workflow) else {
        return Ok(None);
    };
    if bridge.execution_status_key(queued_key).is_err() {
        return Ok(None);
    }
    let mut metadata = parent.metadata.as_object().cloned().unwrap_or_default();
    crate::task_board::apply_workflow_status(workflow, queued_key, &mut metadata)?;
    let event = AppendAgentEventRequest {
        task_id: Some(parent.id.clone()),
        run_id: None,
        worker_id: parent.assigned_to.clone(),
        team_id: parent.team_id.clone(),
        kind: AgentEventKind::TaskResumed,
        from_status: Some(parent.status),
        to_status: Some(AgentTaskStatus::Queued),
        from_status_key: Some(crate::task_board::task_board_status_key(parent)),
        to_status_key: Some(queued_key.into()),
        message: "work plan summary queued".into(),
        payload: json!({"planning_attempt_id": attempt}),
        created_at_ms: now,
    };
    Ok(Some((serde_json::to_string(&metadata)?, event)))
}
