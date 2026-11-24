//! 协调任务就绪检查、运行租约与本地派发的原子流程。

use anyhow::{Context, Result, bail};
use serde_json::Value;
use vw_api_transport::common::TimestampMs;
use vw_api_task::TaskPoolWorkflowSnapshotDto;

use crate::adapters::local::LocalDispatchRequest;
use crate::domain::{RunId, TaskId};
use crate::lifecycle_bridge::builtin_task_workflow_snapshot;
use crate::lifecycle_bridge::{LifecycleBridge, LifecycleDispatchEligibility};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::store::runs::{ClaimReadyTaskRunRequest, StoredAgentRun};
use crate::store::task_plan::{PlanDispatchSnapshot, TaskPlanDispatchBlocker, hidden};
use crate::store::tasks::{AgentTaskFilter, StoredAgentTask, task_dispatch_sort_key};
use crate::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::task_graph::{TaskGraph, TaskGraphReadiness};

pub const DEFAULT_RUN_LEASE_DURATION_MS: i64 = 30 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 成功派发后确定的任务、运行、节点、目标和可选发件箱消息。
pub struct DispatchOutcome {
    pub task_id: TaskId,
    pub run_id: RunId,
    pub target: DispatchTarget,
    pub outbox_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 本地直连派发目标。
pub enum DispatchTarget {
    Local(LocalDispatchRequest),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// 对派发目标的自动选择或强制覆盖策略。
pub enum DispatchMode {
    #[default]
    Auto,
    Local,
}

/// 协调存储、控制器和工作节点选择的单轮派发引擎。
pub struct DispatchEngine<'a> {
    store: &'a AgentSchedulerStore,
    lease_duration_ms: i64,
    dispatch_mode: DispatchMode,
    max_concurrent: Option<usize>,
}

impl<'a> DispatchEngine<'a> {
    pub fn new(store: &'a AgentSchedulerStore) -> Self {
        Self {
            store,
            lease_duration_ms: DEFAULT_RUN_LEASE_DURATION_MS,
            dispatch_mode: DispatchMode::default(),
            max_concurrent: None,
        }
    }

    pub fn with_dispatch_mode(mut self, dispatch_mode: DispatchMode) -> Self {
        self.dispatch_mode = dispatch_mode;
        self
    }

    pub fn with_max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = Some(max_concurrent);
        self
    }

    /// 查找首个就绪任务并尝试派发；没有可派发任务时返回 `Ok(None)`。
    pub async fn dispatch_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
    ) -> Result<Option<DispatchOutcome>> {
        tracing::info!(project_path = %project_path.storage_project_path().display(), dispatch_mode = ?self.dispatch_mode, now_ms = now_ms.0, "agent scheduler dispatch wake started");
        let Some(task) = self.next_pending_task(project_path)? else {
            tracing::info!(project_path = %project_path.storage_project_path().display(), dispatch_mode = ?self.dispatch_mode, "agent scheduler dispatch wake found no queued finite task");
            return Ok(None);
        };
        self.dispatch_task(project_path, task, now_ms).await
    }

    /// 仅尝试指定任务，任务不存在或尚未就绪时返回 `Ok(None)`。
    pub async fn dispatch_task_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        now_ms: TimestampMs,
    ) -> Result<Option<DispatchOutcome>> {
        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task_id.as_str(), dispatch_mode = ?self.dispatch_mode, now_ms = now_ms.0, "agent scheduler task dispatch started");
        let Some(task) = ready_queued_finite_task(self.store, project_path, Some(task_id))? else {
            tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task_id.as_str(), "agent scheduler task dispatch skipped non-ready queued finite task");
            return Ok(None);
        };
        self.dispatch_task(project_path, task, now_ms).await
    }

    async fn dispatch_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: StoredAgentTask,
        now_ms: TimestampMs,
    ) -> Result<Option<DispatchOutcome>> {
        let Some(run) = self.claim_ready_run(project_path, &task, now_ms)? else {
            tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), "agent scheduler task became unavailable before ready claim");
            return Ok(None);
        };
        let target = DispatchTarget::Local(LocalDispatchRequest {
            task_id: task.id.clone(),
            run_id: run.id.clone(),
            requested_at_ms: now_ms,
        });
        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), run_id = run.id.as_str(), "agent scheduler local dispatch claimed");
        Ok(Some(DispatchOutcome {
            task_id: task.id,
            run_id: run.id,
            target,
            outbox_message_id: None,
        }))
    }

    fn next_pending_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Option<StoredAgentTask>> {
        ready_queued_finite_task(self.store, project_path, None)
    }

    fn claim_ready_run(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRun>> {
        let lease_expires_at_ms = TimestampMs(
            now_ms
                .0
                .checked_add(self.lease_duration_ms)
                .context("run lease expiry timestamp overflow")?,
        );
        let run_id = RunId::new(format!("run-{}-{}", task.id.as_str(), now_ms.0))?;
        self.store.claim_ready_task_run_with_limit(
            project_path,
            ClaimReadyTaskRunRequest {
                run_id,
                task_id: task.id.clone(),
                lease_acquired_at_ms: now_ms,
                lease_expires_at_ms: Some(lease_expires_at_ms),
            },
            self.max_concurrent,
        )
    }
}

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
    eligibility.eligible &= !hidden(task.status, &task.metadata);
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
    TaskPlan(TaskPlanDispatchBlocker),
    ExplicitDependency { task_ids: Vec<TaskId> },
}

/// Read model for explaining what blocks new dispatches of queued tasks.
pub struct DispatchReadinessSnapshot {
    workflow: TaskPoolWorkflowSnapshotDto,
    plan: PlanDispatchSnapshot,
    graph: TaskGraph,
}

impl DispatchReadinessSnapshot {
    pub fn load(
        store: &AgentSchedulerStore,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> Result<Self> {
        Ok(Self {
            workflow: load_dispatch_workflow(store, context)?,
            plan: store.load_plan_dispatch_snapshot(context)?,
            graph: store.load_task_graph(context)?,
        })
    }

    pub fn blocker_for(&self, task_id: &TaskId) -> Result<Option<DispatchBlocker>> {
        if let Some(blocker) = self.plan.blocker_for(task_id, &self.workflow) {
            return Ok(Some(DispatchBlocker::TaskPlan(blocker)));
        }
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
