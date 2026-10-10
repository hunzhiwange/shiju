//! 协调任务就绪检查、工作节点选择、运行租约与本地或 Matrix 派发的原子流程。

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use vw_api_types::agent_scheduler::AgentSchedulerMatrixDispatchEventDto;
use vw_api_types::common::TimestampMs;
use vw_api_types::task::TaskPoolWorkflowSnapshotDto;

use crate::agent_scheduler::adapters::controller::{ControllerAdapter, EnsureWorkerReadyRequest};
use crate::agent_scheduler::adapters::local::LocalDispatchRequest;
use crate::agent_scheduler::adapters::matrix::MatrixDispatchTarget;
use crate::agent_scheduler::domain::{RunId, TaskId, WorkerName};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::lifecycle_bridge::{LifecycleBridge, LifecycleDispatchEligibility};
use crate::agent_scheduler::scheduler::worker_selection::{
    WorkerCandidateScore, WorkerSelectionRequest, select_worker_candidates,
};
use crate::agent_scheduler::lifecycle_bridge::builtin_task_workflow_snapshot;
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::events::AppendAgentEventRequest;
use crate::agent_scheduler::store::outbox::EnqueueOutboxMessageRequest;
use crate::agent_scheduler::store::runs::{ClaimReadyTaskRunRequest, StoredAgentRun};
use crate::agent_scheduler::store::tasks::{AgentTaskFilter, StoredAgentTask};
use crate::agent_scheduler::store::workers::{AgentWorkerFilter, StoredAgentWorker};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::agent_scheduler::task_graph::{TaskGraph, TaskGraphReadiness};

pub const DEFAULT_RUN_LEASE_DURATION_MS: i64 = 30 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 成功派发后确定的任务、运行、节点、目标和可选发件箱消息。
pub struct DispatchOutcome {
    pub task_id: TaskId,
    pub run_id: RunId,
    pub worker_id: WorkerName,
    pub target: DispatchTarget,
    pub outbox_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 本地直连或 Matrix 异步消息两类派发目标。
pub enum DispatchTarget {
    Local(LocalDispatchRequest),
    Matrix(MatrixDispatchTarget),
}

impl DispatchTarget {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Local(_) => "local",
            Self::Matrix(_) => "matrix",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// 对派发目标的自动选择或强制覆盖策略。
pub enum DispatchMode {
    #[default]
    Auto,
    Local,
    Matrix,
}

/// 协调存储、控制器和工作节点选择的单轮派发引擎。
pub struct DispatchEngine<'a, C>
where
    C: ControllerAdapter + ?Sized,
{
    store: &'a AgentSchedulerStore,
    controller: &'a C,
    lease_duration_ms: i64,
    dispatch_mode: DispatchMode,
}

impl<'a, C> DispatchEngine<'a, C>
where
    C: ControllerAdapter + ?Sized,
{
    pub fn new(store: &'a AgentSchedulerStore, controller: &'a C) -> Self {
        Self {
            store,
            controller,
            lease_duration_ms: DEFAULT_RUN_LEASE_DURATION_MS,
            dispatch_mode: DispatchMode::default(),
        }
    }

    pub fn with_dispatch_mode(mut self, dispatch_mode: DispatchMode) -> Self {
        self.dispatch_mode = dispatch_mode;
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
        let selection = selection_request_from_task(&task)?;
        let workers = self.store.list_workers(project_path, AgentWorkerFilter::default())?;
        let active_tasks =
            self.store.list_active_tasks(project_path, AgentTaskFilter::default())?;
        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), dispatch_mode = ?self.dispatch_mode, workers = workers.len(), active_tasks = active_tasks.len(), required_skills = ?selection.required_skills, preferred_runtime = ?selection.preferred_runtime, team_id = ?selection.team_id.as_ref().map(|team_id| team_id.as_str()), "agent scheduler selecting worker for task");
        let Some(candidate) =
            select_worker_candidates(&selection, &workers, &active_tasks).into_iter().next()
        else {
            tracing::warn!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), dispatch_mode = ?self.dispatch_mode, workers = workers.len(), active_tasks = active_tasks.len(), "agent scheduler dispatch found no eligible worker");
            return Ok(None);
        };
        let worker = selected_worker(&workers, &candidate)?;
        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), worker_id = worker.id.as_str(), worker_runtime = ?worker.runtime, worker_room_id = ?worker.room_id, worker_matrix_user_id = ?worker.matrix_user_id, worker_availability = ?candidate.availability, finite_active_tasks = candidate.finite_active_tasks, infinite_active_tasks = candidate.infinite_active_tasks, active_tasks = candidate.active_tasks, runtime_matched = candidate.runtime_matched, can_dispatch_now = candidate.can_dispatch_now, requires_wakeup = candidate.requires_wakeup, "agent scheduler selected worker");

        let ready = self
            .controller
            .ensure_worker_ready(EnsureWorkerReadyRequest {
                worker_name: worker.id.clone(),
                runtime: worker.runtime.clone(),
                team_id: worker.team_id.clone(),
                worker_role: worker.worker_role.clone(),
            })
            .await?;
        if ready.worker_name != worker.id {
            bail!(
                "controller returned readiness for {}, expected {}",
                ready.worker_name.as_str(),
                worker.id.as_str()
            );
        }
        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), worker_id = worker.id.as_str(), ready_runtime = ?ready.runtime, ready_room_id = ?ready.room_id, dispatch_mode = ?self.dispatch_mode, "agent scheduler worker ready");
        validate_dispatch_mode_available(&ready, &worker, self.dispatch_mode)?;

        let Some(run) = self.claim_ready_run(project_path, &task, &worker, now_ms)? else {
            tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = task.id.as_str(), "agent scheduler task became unavailable before ready claim");
            return Ok(None);
        };
        let dispatched = self
            .store
            .transition_task_status(project_path, &task.id, AgentTaskStatus::Dispatched, now_ms)?
            .with_context(|| {
                format!("assigned task disappeared before dispatch: {}", task.id.as_str())
            })?;

        let target = dispatch_target_from_worker(
            &ready,
            &worker,
            &dispatched,
            &run,
            now_ms,
            self.dispatch_mode,
        )?;
        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = dispatched.id.as_str(), run_id = run.id.as_str(), worker_id = worker.id.as_str(), dispatch_target = target.as_str(), "agent scheduler dispatch target resolved");
        self.append_worker_selected_event(project_path, &task, &worker, &run, &target, now_ms)?;

        let outbox_message_id = match &target {
            DispatchTarget::Matrix(matrix_target) => {
                let outbox = self.enqueue_dispatch_message(
                    project_path,
                    &dispatched,
                    &worker,
                    &run,
                    matrix_target,
                    now_ms,
                )?;
                tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = dispatched.id.as_str(), run_id = run.id.as_str(), worker_id = worker.id.as_str(), room_id = matrix_target.room_id.as_str(), recipient_id = matrix_target.recipient_id.as_str(), outbox_message_id = outbox.id.as_str(), "agent scheduler queued Matrix dispatch outbox message");
                self.append_matrix_dispatched_event(
                    project_path,
                    &dispatched,
                    &worker,
                    &run,
                    &outbox.id,
                    now_ms,
                )?;
                Some(outbox.id)
            }
            DispatchTarget::Local(_) => None,
        };

        tracing::info!(project_path = %project_path.storage_project_path().display(), task_id = dispatched.id.as_str(), run_id = run.id.as_str(), worker_id = worker.id.as_str(), dispatch_target = target.as_str(), outbox_message_id = ?outbox_message_id, "agent scheduler dispatch completed");
        Ok(Some(DispatchOutcome {
            task_id: dispatched.id,
            run_id: run.id,
            worker_id: worker.id,
            target,
            outbox_message_id,
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
        worker: &StoredAgentWorker,
        now_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRun>> {
        let lease_expires_at_ms = TimestampMs(
            now_ms
                .0
                .checked_add(self.lease_duration_ms)
                .context("run lease expiry timestamp overflow")?,
        );
        let run_id = RunId::new(format!("run-{}-{}", task.id.as_str(), now_ms.0))?;
        self.store.claim_ready_task_run(
            project_path,
            ClaimReadyTaskRunRequest {
                run_id,
                task_id: task.id.clone(),
                worker_id: worker.id.clone(),
                lease_acquired_at_ms: now_ms,
                lease_expires_at_ms: Some(lease_expires_at_ms),
            },
        )
    }

    fn append_worker_selected_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        worker: &StoredAgentWorker,
        run: &StoredAgentRun,
        target: &DispatchTarget,
        now_ms: TimestampMs,
    ) -> Result<()> {
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: Some(run.id.clone()),
                worker_id: Some(worker.id.clone()),
                team_id: task.team_id.clone(),
                kind: AgentEventKind::WorkerSelected,
                from_status: Some(AgentTaskStatus::Queued),
                to_status: Some(AgentTaskStatus::Assigned),
                from_status_key: None,
                to_status_key: None,
                message: format!(
                    "selected worker {} for task {}",
                    worker.id.as_str(),
                    task.id.as_str()
                ),
                payload: json!({
                    "task_id": task.id.as_str(),
                    "worker_id": worker.id.as_str(),
                    "run_id": run.id.as_str(),
                    "dispatch_target": target.as_str(),
                }),
                created_at_ms: now_ms,
            },
        )?;
        Ok(())
    }

    fn enqueue_dispatch_message(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        worker: &StoredAgentWorker,
        run: &StoredAgentRun,
        target: &MatrixDispatchTarget,
        now_ms: TimestampMs,
    ) -> Result<crate::agent_scheduler::store::outbox::StoredOutboxMessage> {
        let message_id = format!("outbox-{}-dispatch-{}", task.id.as_str(), worker.id.as_str());
        let mention = target.recipient_id.as_str();
        let body = format!(
            "{mention} New task [{}]: {}. Use your file-sync skill to pull the spec: {}. @mention me when complete.",
            task.id.as_str(),
            task.title,
            task.spec_uri
        );
        let payload = matrix_dispatch_event(project_path, task, worker, run, target, now_ms)?;
        payload.validate_for_matrix_room_event().map_err(anyhow::Error::msg)?;
        self.store.enqueue_outbox_message(
            project_path,
            EnqueueOutboxMessageRequest {
                message_id,
                channel: "matrix".to_string(),
                target: format!("room:{}", target.room_id),
                recipient_id: Some(target.recipient_id.clone()),
                body,
                idempotency_key: format!("{}:dispatch:{}", task.id.as_str(), worker.id.as_str()),
                payload: serde_json::to_value(payload)
                    .context("serialize Matrix dispatch event payload")?,
                created_at_ms: now_ms,
                next_attempt_at_ms: None,
            },
        )
    }

    fn append_matrix_dispatched_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        worker: &StoredAgentWorker,
        run: &StoredAgentRun,
        outbox_message_id: &str,
        now_ms: TimestampMs,
    ) -> Result<()> {
        self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: Some(run.id.clone()),
                worker_id: Some(worker.id.clone()),
                team_id: task.team_id.clone(),
                kind: AgentEventKind::MatrixDispatched,
                from_status: Some(AgentTaskStatus::Assigned),
                to_status: Some(AgentTaskStatus::Dispatched),
                from_status_key: None,
                to_status_key: None,
                message: format!(
                    "queued Matrix dispatch for task {} to {}",
                    task.id.as_str(),
                    worker.id.as_str()
                ),
                payload: json!({
                    "task_id": task.id.as_str(),
                    "worker_id": worker.id.as_str(),
                    "run_id": run.id.as_str(),
                    "outbox_message_id": outbox_message_id,
                }),
                created_at_ms: now_ms,
            },
        )?;
        Ok(())
    }
}

pub(crate) fn load_dispatch_workflow(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<TaskPoolWorkflowSnapshotDto> {
    Ok(store
        .load_task_board_workflow(project_path)?
        .unwrap_or_else(builtin_task_workflow_snapshot))
}

pub(crate) fn task_lifecycle_allows_dispatch(
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

pub(crate) fn task_lifecycle_dispatch_eligibility(
    task: &StoredAgentTask,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Result<LifecycleDispatchEligibility> {
    let status_key = dispatch_status_key(task);
    let eligibility = LifecycleBridge::new(workflow).dispatch_eligibility(&status_key)?;
    if eligibility.status.runtime_status != task.status {
        bail!(
            "workflow status `{}` maps to runtime `{}`, task {} has runtime `{}`",
            eligibility.status.status_key,
            eligibility.status.runtime_status.as_str(),
            task.id.as_str(),
            task.status.as_str()
        );
    }
    Ok(eligibility)
}

pub(crate) fn dispatch_status_key(task: &StoredAgentTask) -> String {
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

pub(crate) fn legacy_dispatch_status_key(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Queued => "pending",
        AgentTaskStatus::Submitted => "code_complete",
        _ => status.as_str(),
    }
}

pub(crate) fn dispatch_target_from_worker(
    ready: &crate::agent_scheduler::adapters::controller::WorkerControllerSnapshot,
    worker: &StoredAgentWorker,
    task: &StoredAgentTask,
    run: &StoredAgentRun,
    now_ms: TimestampMs,
    dispatch_mode: DispatchMode,
) -> Result<DispatchTarget> {
    let room_id = ready.room_id.clone().or_else(|| worker.room_id.clone());
    let local_target = || {
        DispatchTarget::Local(LocalDispatchRequest {
            task_id: task.id.clone(),
            run_id: run.id.clone(),
            worker_id: worker.id.clone(),
            requested_at_ms: now_ms,
        })
    };
    match dispatch_mode {
        DispatchMode::Local => Ok(local_target()),
        DispatchMode::Auto => match (room_id, worker.matrix_user_id.clone()) {
            (Some(room_id), Some(recipient_id)) => {
                Ok(DispatchTarget::Matrix(MatrixDispatchTarget { room_id, recipient_id }))
            }
            _ => Ok(local_target()),
        },
        DispatchMode::Matrix => match (room_id, worker.matrix_user_id.clone()) {
            (Some(room_id), Some(recipient_id)) => {
                Ok(DispatchTarget::Matrix(MatrixDispatchTarget { room_id, recipient_id }))
            }
            (None, _) => {
                bail!("Matrix dispatch requires room_id for worker {}", worker.id.as_str())
            }
            (_, None) => {
                bail!("Matrix dispatch requires matrix_user_id for worker {}", worker.id.as_str())
            }
        },
    }
}

pub(crate) fn validate_dispatch_mode_available(
    ready: &crate::agent_scheduler::adapters::controller::WorkerControllerSnapshot,
    worker: &StoredAgentWorker,
    dispatch_mode: DispatchMode,
) -> Result<()> {
    if dispatch_mode != DispatchMode::Matrix {
        return Ok(());
    }
    let room_id = ready.room_id.as_ref().or(worker.room_id.as_ref());
    if room_id.is_none() {
        bail!("Matrix dispatch requires room_id for worker {}", worker.id.as_str());
    }
    if worker.matrix_user_id.is_none() {
        bail!("Matrix dispatch requires matrix_user_id for worker {}", worker.id.as_str());
    }
    Ok(())
}

pub(crate) fn selected_worker(
    workers: &[StoredAgentWorker],
    candidate: &WorkerCandidateScore,
) -> Result<StoredAgentWorker> {
    workers.iter().find(|worker| worker.id == candidate.worker_id).cloned().with_context(|| {
        format!("selected worker snapshot missing: {}", candidate.worker_id.as_str())
    })
}

pub(crate) fn ready_queued_finite_task(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task_id: Option<&TaskId>,
) -> Result<Option<StoredAgentTask>> {
    let workflow = load_dispatch_workflow(store, project_path)?;
    let graph = load_task_graph(store, project_path)?;
    let filter = AgentTaskFilter {
        task_id: task_id.cloned(),
        status: Some(AgentTaskStatus::Queued),
        assigned_to: None,
    };
    let tasks = store.list_active_tasks(project_path, filter)?;
    for task in tasks {
        if task.kind != AgentTaskKind::Finite
            || !task_lifecycle_allows_dispatch(project_path, &task, &workflow)
            || !task_graph_allows_dispatch(&graph, &task)?
        {
            continue;
        }
        return Ok(Some(task));
    }
    Ok(None)
}

fn load_task_graph(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<TaskGraph> {
    store.load_task_graph(project_path)
}

fn task_graph_allows_dispatch(graph: &TaskGraph, task: &StoredAgentTask) -> Result<bool> {
    if !graph.dependencies().iter().any(|dependency| dependency.successor == task.id) {
        return Ok(true);
    }
    Ok(matches!(graph.readiness_for(&task.id)?, TaskGraphReadiness::Ready))
}

pub(crate) fn matrix_dispatch_event(
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
    worker: &StoredAgentWorker,
    run: &StoredAgentRun,
    target: &MatrixDispatchTarget,
    now_ms: TimestampMs,
) -> Result<AgentSchedulerMatrixDispatchEventDto> {
    let acp_agent = dispatch_acp_agent(task);
    let reasoning_effort = if acp_agent.is_some() {
        crate::agent_scheduler::task_board::reasoning_effort_from_metadata(&task.metadata)?
    } else {
        None
    };
    let scope = project_path.storage_scope()?;
    let scope = scope.project_scope();
    Ok(AgentSchedulerMatrixDispatchEventDto {
        event_version: 1,
        platform_id: scope.platform_id().as_ref().to_string(),
        company_id: scope.company_id().as_ref().to_string(),
        project_id: scope.project_id().as_ref().to_string(),
        task_id: task.id.as_str().to_string(),
        run_id: run.id.as_str().to_string(),
        worker_id: worker.id.as_str().to_string(),
        worker_role: worker.worker_role.clone().unwrap_or_else(|| "worker".to_string()),
        room_id: target.room_id.clone(),
        matrix_user_id: target.recipient_id.clone(),
        model: dispatch_model(task).unwrap_or_else(|| worker.model.clone()),
        model_provider: worker.model_provider.clone(),
        agent: dispatch_agent(task, acp_agent.as_deref()),
        acp_agent,
        reasoning_effort,
        spec_path: task.spec_uri.clone(),
        metadata_path: task.meta_uri.clone(),
        created_at_ms: now_ms,
    })
}

pub(crate) fn dispatch_model(task: &StoredAgentTask) -> Option<String> {
    dispatch_metadata_string(task, "model").filter(|model| !model.eq_ignore_ascii_case("auto"))
}

pub(crate) fn dispatch_agent(task: &StoredAgentTask, acp_agent: Option<&str>) -> Option<String> {
    let agent = dispatch_metadata_string(task, "agent")?;
    if acp_agent.is_some() && agent.eq_ignore_ascii_case("main") {
        return None;
    }
    Some(agent)
}

pub(crate) fn dispatch_acp_agent(task: &StoredAgentTask) -> Option<String> {
    dispatch_metadata_string(task, "acp_agent").and_then(normalize_dispatch_acp_agent)
}

pub(crate) fn normalize_dispatch_acp_agent(value: String) -> Option<String> {
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

pub(crate) fn selection_request_from_task(
    task: &StoredAgentTask,
) -> Result<WorkerSelectionRequest> {
    Ok(WorkerSelectionRequest {
        required_skills: required_skills_from_metadata(&task.metadata)?,
        team_id: task.team_id.clone(),
        preferred_runtime: preferred_runtime_from_metadata(&task.metadata)?,
    })
}

fn required_skills_from_metadata(metadata: &Value) -> Result<Vec<String>> {
    let skills = string_array_field(metadata, "required_skills")?;
    if skills.is_empty() {
        return string_array_field(metadata, "skills");
    }
    Ok(skills)
}

fn string_array_field(metadata: &Value, field_name: &str) -> Result<Vec<String>> {
    let Some(value) = metadata.get(field_name) else {
        return Ok(Vec::new());
    };
    let Value::Array(values) = value else {
        bail!("task metadata field {field_name} must be an array of strings");
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .with_context(|| format!("task metadata field {field_name} must contain strings"))
        })
        .collect()
}

fn preferred_runtime_from_metadata(metadata: &Value) -> Result<Option<String>> {
    let Some(value) = metadata.get("preferred_runtime").or_else(|| metadata.get("runtime")) else {
        return Ok(None);
    };
    let runtime = value
        .as_str()
        .with_context(|| "task metadata field preferred_runtime must be a string")?
        .trim();
    if runtime.is_empty() {
        return Ok(None);
    }
    Ok(Some(runtime.to_string()))
}
