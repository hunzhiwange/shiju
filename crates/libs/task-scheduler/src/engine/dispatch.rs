//! 检查任务就绪状态和本地派发条件。

use anyhow::{Context, Result, bail};
use serde_json::Value;
use vw_api_task::TaskPoolWorkflowSnapshotDto;

use crate::domain::TaskId;
use crate::lifecycle_bridge::builtin_task_workflow_snapshot;
use crate::lifecycle_bridge::{LifecycleBridge, LifecycleDispatchEligibility};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::store::tasks::{AgentTaskFilter, StoredAgentTask, bool_metadata_field, task_dispatch_sort_key};
use crate::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::task_graph::{TaskGraph, TaskGraphReadiness};

pub fn load_dispatch_workflow(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<TaskPoolWorkflowSnapshotDto> {
    effective_dispatch_workflow(store.load_task_board_workflow(project_path)?)
}

pub(crate) fn effective_dispatch_workflow(
    workflow: Option<TaskPoolWorkflowSnapshotDto>,
) -> Result<TaskPoolWorkflowSnapshotDto> {
    let workflow = workflow.unwrap_or_else(builtin_task_workflow_snapshot);
    workflow.validate_for_schedule().map_err(anyhow::Error::msg)
        .with_context(|| format!("invalid project workflow `{}`", workflow.id))?;
    Ok(workflow)
}

pub fn task_lifecycle_allows_dispatch(
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> bool {
    match task_lifecycle_dispatch_eligibility(task, workflow) {
        Ok(eligibility) if eligibility.eligible => true,
        Ok(eligibility) => {
            tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), task_status = task.status.as_str(), status_key = eligibility.status.status_key.as_str(), status_role = ?eligibility.status.status_role, projected_runtime_status = eligibility.status.runtime_status.as_str(), "agent scheduler dispatch skipped lifecycle-ineligible queued task");
            false
        }
        Err(error) => {
            tracing::warn!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), task_status = task.status.as_str(), error = %error, "agent scheduler dispatch skipped task with invalid lifecycle projection");
            false
        }
    }
}

pub fn task_lifecycle_dispatch_eligibility(
    task: &StoredAgentTask,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Result<LifecycleDispatchEligibility> {
    let status_key = dispatch_status_key(task);
    let mut eligibility = LifecycleBridge::new(workflow).dispatch_eligibility(&status_key)?;
    if eligibility.status.runtime_status != task.status {
        bail!(
            "workflow status `{}` maps to runtime `{}`, task {} has runtime `{}`",
            eligibility.status.status_key,
            eligibility.status.runtime_status.as_str(),
            task.id.as_str(),
            task.status.as_str()
        );
    }
    eligibility.eligible &= task.status != AgentTaskStatus::Archived
        && !bool_metadata_field(&task.metadata, "task_board.archived")
        && !bool_metadata_field(&task.metadata, "task_board.deleted");
    Ok(eligibility)
}

pub fn dispatch_status_key(task: &StoredAgentTask) -> String {
    metadata_non_empty_string(&task.metadata, "task_board.status_key")
        .or_else(|| metadata_status_key_map_value(&task.metadata, task.status))
        .unwrap_or_else(|| legacy_dispatch_status_key(task.status).to_string())
}

fn metadata_non_empty_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn metadata_status_key_map_value(metadata: &Value, status: AgentTaskStatus) -> Option<String> {
    metadata
        .get("task_board.status_key_map")
        .and_then(Value::as_object)?
        .get(status.as_str())
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn legacy_dispatch_status_key(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Draft => "pool",
        AgentTaskStatus::Queued => "pending",
        AgentTaskStatus::Submitted => "code_complete",
        _ => status.as_str(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchBlocker {
    ExplicitDependency { task_ids: Vec<TaskId> },
}

/// Read model for explaining what blocks new dispatches of queued tasks.
pub struct DispatchReadinessSnapshot {
    workflow: TaskPoolWorkflowSnapshotDto,
    graph: TaskGraph,
}

impl DispatchReadinessSnapshot {
    pub fn load(
        store: &AgentSchedulerStore,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Self> {
        Ok(Self {
            workflow: load_dispatch_workflow(store, context)?,
            graph: store.load_task_graph(context)?,
        })
    }

    pub fn blocker_for(&self, task_id: &TaskId) -> Result<Option<DispatchBlocker>> {
        if !self.graph.dependencies().iter().any(|edge| &edge.successor == task_id) {
            return Ok(None);
        }
        match self.graph.readiness_for(task_id)? {
            TaskGraphReadiness::WaitingDependencies { task_ids }
            | TaskGraphReadiness::BlockedByDependency { task_ids } => {
                Ok(Some(DispatchBlocker::ExplicitDependency { task_ids }))
            }
            TaskGraphReadiness::Ready | TaskGraphReadiness::NotQueued => Ok(None),
        }
    }
}

pub fn ready_queued_finite_task(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: Option<&TaskId>,
) -> Result<Option<StoredAgentTask>> {
    let snapshot = DispatchReadinessSnapshot::load(store, project_path)?;
    let filter = AgentTaskFilter {
        task_id: task_id.cloned(),
        status: Some(AgentTaskStatus::Queued),
        assigned_to: None,
    };
    let mut tasks = store.list_active_tasks(project_path, filter)?;
    tasks.sort_by_cached_key(task_dispatch_sort_key);
    for task in tasks {
        if task.kind != AgentTaskKind::Finite
            || !task_lifecycle_allows_dispatch(project_path, &task, &snapshot.workflow)
            || snapshot.blocker_for(&task.id)?.is_some()
        {
            continue;
        }
        return Ok(Some(task));
    }
    Ok(None)
}

pub fn dispatch_model(task: &StoredAgentTask) -> Option<String> {
    dispatch_metadata_string(task, "model").filter(|model| !model.eq_ignore_ascii_case("auto"))
}

pub fn dispatch_agent(task: &StoredAgentTask, acp_agent: Option<&str>) -> Option<String> {
    let agent = dispatch_metadata_string(task, "agent")?;
    if acp_agent.is_some() && agent.eq_ignore_ascii_case("main") {
        return None;
    }
    Some(agent)
}

pub fn dispatch_acp_agent(task: &StoredAgentTask) -> Option<String> {
    dispatch_metadata_string(task, "acp_agent").and_then(normalize_dispatch_acp_agent)
}

pub fn normalize_dispatch_acp_agent(value: String) -> Option<String> {
    match value.to_ascii_lowercase().as_str() {
        "default" | "acp" | "internal" => None,
        _ => Some(value),
    }
}

fn dispatch_metadata_string(task: &StoredAgentTask, key: &str) -> Option<String> {
    task.metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
