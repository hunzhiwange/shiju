//! 测试调度引擎分派就绪任务时的选员、租约与失败回滚。
//! 竞争领取和执行失败夹具确认任务只分派一次，失败后状态与租约同步收敛。

use anyhow::Result;
use serde_json::json;
use vw_api_types::ReasoningEffortDto;
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
};

use super::dispatch::{
    DEFAULT_RUN_LEASE_DURATION_MS, DispatchEngine, DispatchMode, DispatchTarget,
    load_dispatch_workflow,
};
use crate::agent_scheduler::adapters::controller::{
    ControllerAdapter, EnsureWorkerReadyRequest, FakeControllerAdapter, WorkerControllerSnapshot,
    WorkerLifecycleRequest, WorkerLifecycleState, WorkerStatusRequest,
};
use crate::agent_scheduler::domain::{AgentWorkerStatus, RunId, TaskId, TeamId, WorkerName};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::scheduler::worker_selection::{
    WorkerAvailability, WorkerCandidateScore,
};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::runs::ClaimTaskRunRequest;
use crate::agent_scheduler::store::tasks::NewAgentTask;
use crate::agent_scheduler::store::workers::{AgentCapabilityRecord, NewAgentWorkerSnapshot};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::agent_scheduler::task_graph::AgentTaskDependency;

struct EnabledTraceSubscriber;

#[test]
fn dispatch_workflow_fallback_does_not_require_staff_config() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(temp_dir.path())?;

    let workflow = load_dispatch_workflow(&store, temp_dir.path())?;

    assert_eq!(
        workflow,
        crate::agent_scheduler::lifecycle_bridge::builtin_task_workflow_snapshot()
    );
    assert!(
        workflow
            .statuses
            .iter()
            .any(|status| status.role == TaskPoolWorkflowStatusRoleDto::Queued)
    );
    Ok(())
}

impl tracing::Subscriber for EnabledTraceSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {}

    fn enter(&self, _span: &tracing::Id) {}

    fn exit(&self, _span: &tracing::Id) {}
}


#[tokio::test]
async fn dispatch_once_returns_none_when_no_queued_finite_task_exists() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    store.initialize_project(project_path)?;

    let outcome = DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, TimestampMs(1))
        .await?;

    assert_eq!(outcome, None);
    assert!(controller.recorded_lifecycle_requests().is_empty());
    Ok(())
}

#[tokio::test]
async fn local_dispatch_claims_ready_task_without_worker_snapshot() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;

    let outcome = DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, TimestampMs(1_783_000_305_000))
        .await?;

    assert!(outcome.is_some(), "ready task should be claimed without a worker snapshot");
    assert!(controller.recorded_lifecycle_requests().is_empty());
    Ok(())
}

#[tokio::test]
async fn dispatch_task_once_skips_missing_and_non_queued_finite_tasks() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    store.initialize_project(project_path)?;

    let missing = DispatchEngine::new(&store, &controller)
        .dispatch_task_once(project_path, &TaskId::new("missing-task")?, TimestampMs(2))
        .await?;
    assert_eq!(missing, None);

    let mut running = pending_task()?;
    running.status = AgentTaskStatus::Running;
    store.create_task(project_path, running)?;
    let non_queued = DispatchEngine::new(&store, &controller)
        .dispatch_task_once(project_path, &TaskId::new("task-dispatch-1")?, TimestampMs(3))
        .await?;

    assert_eq!(non_queued, None);
    assert!(controller.recorded_lifecycle_requests().is_empty());
    Ok(())
}


#[tokio::test]
async fn dispatch_returns_none_when_no_worker_matches_task_requirements() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;

    let outcome = DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, TimestampMs(4))
        .await?;

    assert_eq!(outcome, None);
    assert!(controller.recorded_lifecycle_requests().is_empty());
    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    Ok(())
}

#[tokio::test]
async fn dispatch_rejects_controller_ready_snapshot_for_different_worker() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = WrongReadyController;
    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;
    store.upsert_worker_snapshot(project_path, sleeping_worker()?)?;

    let error = DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, TimestampMs(5))
        .await
        .expect_err("controller must report readiness for the selected worker");

    assert!(error.to_string().contains("controller returned readiness for worker-other"));
    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    Ok(())
}

#[test]
fn dispatch_tracing_enabled_evaluates_selection_and_completion_fields() -> Result<()> {
    let dispatch = tracing::Dispatch::new(EnabledTraceSubscriber);
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    let no_worker_dir = tempfile::tempdir()?;
    let no_worker_project = no_worker_dir.path();
    let no_worker_store = AgentSchedulerStore::new();
    let no_worker_controller = FakeControllerAdapter::new();
    no_worker_store.initialize_project(no_worker_project)?;
    no_worker_store.create_task(no_worker_project, pending_task()?)?;
    tracing::dispatcher::with_default(&dispatch, || {
        runtime.block_on(async {
            let outcome = DispatchEngine::new(&no_worker_store, &no_worker_controller)
                .dispatch_once(no_worker_project, TimestampMs(6))
                .await?;
            assert_eq!(outcome, None);
            Result::<()>::Ok(())
        })
    })?;

    let success_dir = tempfile::tempdir()?;
    let success_project = success_dir.path();
    let success_store = AgentSchedulerStore::new();
    let success_controller = FakeControllerAdapter::new();
    success_store.initialize_project(success_project)?;
    success_store.create_task(success_project, pending_task()?)?;
    success_store.upsert_worker_snapshot(success_project, sleeping_worker()?)?;
    success_controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });
    tracing::dispatcher::with_default(&dispatch, || {
        runtime.block_on(async {
            let outcome = DispatchEngine::new(&success_store, &success_controller)
                .dispatch_once(success_project, TimestampMs(7))
                .await?
                .expect("task dispatches with tracing enabled");
            assert!(outcome.outbox_message_id.is_some());
            Result::<()>::Ok(())
        })
    })?;

    Ok(())
}

#[tokio::test]
async fn dispatch_skips_queued_task_when_lifecycle_stage_is_not_dispatchable() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_350_000);

    store.initialize_project(project_path)?;
    store.save_task_board_workflow(
        project_path,
        &workflow(vec![
            workflow_status("disabled_queue", TaskPoolWorkflowStatusRoleDto::Queued, false),
            workflow_status("review", TaskPoolWorkflowStatusRoleDto::Review, true),
        ]),
    )?;
    store.create_task(
        project_path,
        pending_task_with_status_key("task-disabled", "disabled_queue")?,
    )?;
    store.create_task(project_path, pending_task_with_status_key("task-review", "review")?)?;
    store.upsert_worker_snapshot(project_path, sleeping_worker()?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let engine = DispatchEngine::new(&store, &controller);
    let outcome = engine.dispatch_once(project_path, now_ms).await?;

    assert_eq!(outcome, None);
    assert!(controller.recorded_lifecycle_requests().is_empty());
    let disabled = store.load_task(project_path, &TaskId::new("task-disabled")?)?.expect("task");
    let review = store.load_task(project_path, &TaskId::new("task-review")?)?.expect("task");
    assert_eq!(disabled.status, AgentTaskStatus::Queued);
    assert_eq!(review.status, AgentTaskStatus::Queued);

    Ok(())
}

#[tokio::test]
async fn dispatch_skips_queued_task_with_unsatisfied_dependencies() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_355_000);

    store.initialize_project(project_path)?;
    let mut predecessor = pending_task()?;
    predecessor.id = TaskId::new("task-dispatch-prerequisite")?;
    predecessor.status = AgentTaskStatus::Submitted;
    store.create_task(project_path, predecessor)?;
    let mut blocked = pending_task()?;
    blocked.id = TaskId::new("task-dispatch-blocked")?;
    store.create_task(project_path, blocked)?;
    let mut ready = pending_task()?;
    ready.id = TaskId::new("task-dispatch-ready")?;
    store.create_task(project_path, ready)?;
    store.replace_task_dependencies(
        project_path,
        vec![AgentTaskDependency::new(
            TaskId::new("task-dispatch-prerequisite")?,
            TaskId::new("task-dispatch-blocked")?,
        )],
    )?;
    store.upsert_worker_snapshot(project_path, sleeping_worker()?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let engine = DispatchEngine::new(&store, &controller);
    let directed = engine
        .dispatch_task_once(project_path, &TaskId::new("task-dispatch-blocked")?, now_ms)
        .await?;
    assert_eq!(directed, None);

    let outcome = engine
        .dispatch_once(project_path, now_ms)
        .await?
        .expect("ready task dispatches after blocked task is skipped");

    assert_eq!(outcome.task_id.as_str(), "task-dispatch-ready");
    let blocked = store
        .load_task(project_path, &TaskId::new("task-dispatch-blocked")?)?
        .expect("blocked task persists");
    assert_eq!(blocked.status, AgentTaskStatus::Queued);
    Ok(())
}

#[test]
fn lifecycle_allows_dispatch_reports_non_dispatchable_projection() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    let mut task = pending_task_with_status_key("task-backlog", "backlog")?;
    task.status = AgentTaskStatus::Draft;
    store.create_task(project_path, task)?;
    let stored_task =
        store.load_task(project_path, &TaskId::new("task-backlog")?)?.expect("task stored");
    let workflow =
        workflow(vec![workflow_status("backlog", TaskPoolWorkflowStatusRoleDto::Backlog, true)]);

    assert!(!super::dispatch::task_lifecycle_allows_dispatch(
        project_path,
        &stored_task,
        &workflow
    ));
    Ok(())
}




#[tokio::test]
async fn agent_dispatch_engine_forces_local_target_when_configured() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_500_000);

    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;
    store.upsert_worker_snapshot(project_path, sleeping_worker()?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let engine = DispatchEngine::new(&store, &controller).with_dispatch_mode(DispatchMode::Local);
    let outcome =
        engine.dispatch_once(project_path, now_ms).await?.expect("pending task dispatches");

    assert!(matches!(outcome.target, DispatchTarget::Local(_)));
    assert!(outcome.outbox_message_id.is_none());
    assert!(store.claim_outbox_batch(project_path, TimestampMs(now_ms.0 + 1), 10)?.is_empty());

    Ok(())
}


#[tokio::test]
async fn agent_dispatch_engine_rejects_non_array_required_skills() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let mut task = pending_task()?;
    task.metadata["required_skills"] = json!("api-implementation");

    store.initialize_project(project_path)?;
    store.create_task(project_path, task)?;

    let error = DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, TimestampMs(1_783_000_700_000))
        .await
        .expect_err("required skills must be an array");

    assert!(
        error
            .to_string()
            .contains("task metadata field required_skills must be an array of strings")
    );
    assert!(controller.recorded_lifecycle_requests().is_empty());
    Ok(())
}

#[tokio::test]
async fn agent_dispatch_engine_rejects_non_string_preferred_runtime() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let mut task = pending_task()?;
    task.metadata["preferred_runtime"] = json!(42);

    store.initialize_project(project_path)?;
    store.create_task(project_path, task)?;

    let error = DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, TimestampMs(1_783_000_710_000))
        .await
        .expect_err("preferred runtime must be a string");

    assert!(error.to_string().contains("task metadata field preferred_runtime must be a string"));
    assert!(controller.recorded_lifecycle_requests().is_empty());
    Ok(())
}



fn pending_task() -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new("task-dispatch-1")?,
        title: "Implement dispatch engine".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: "shared/tasks/task-dispatch-1/spec.md".to_string(),
        meta_uri: "shared/tasks/task-dispatch-1/meta.json".to_string(),
        assigned_to: None,
        team_id: Some(TeamId::new("team-alpha")?),
        room_id: None,
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "acp_agent": "claude",
            "agent": "codex",
            "reasoning_effort": "max",
            "required_skills": ["api-implementation"],
            "preferred_runtime": "copaw",
        }),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn pending_task_with_status_key(task_id: &str, status_key: &str) -> Result<NewAgentTask> {
    let mut task = pending_task()?;
    task.id = TaskId::new(task_id)?;
    task.metadata["task_board.status_key"] = json!(status_key);
    Ok(task)
}

fn workflow(statuses: Vec<TaskPoolWorkflowStatusDto>) -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: "dispatch_workflow".to_string(),
        label: "Dispatch Workflow".to_string(),
        statuses,
    }
}

fn workflow_status(
    key: &str,
    role: TaskPoolWorkflowStatusRoleDto,
    enabled: bool,
) -> TaskPoolWorkflowStatusDto {
    TaskPoolWorkflowStatusDto {
        key: key.to_string(),
        label: key.to_string(),
        role,
        enabled,
        terminal: false,
        next: None,
        transitions: Vec::new(),
    }
}



struct WrongReadyController;

#[async_trait::async_trait]
impl ControllerAdapter for WrongReadyController {
    async fn ensure_worker_ready(
        &self,
        request: EnsureWorkerReadyRequest,
    ) -> Result<WorkerControllerSnapshot> {
        Ok(WorkerControllerSnapshot {
            worker_name: WorkerName::new("worker-other")?,
            desired_state: WorkerLifecycleState::Running,
            runtime: request.runtime,
            team_id: request.team_id,
            worker_role: request.worker_role,
            room_id: Some("!wrong-room:example.test".to_string()),
            container_state: Some("ready".to_string()),
        })
    }

    async fn request_worker_sleep(
        &self,
        _request: WorkerLifecycleRequest,
    ) -> Result<WorkerControllerSnapshot> {
        unreachable!("dispatch tests only ensure worker readiness")
    }

    async fn request_worker_stop(
        &self,
        _request: WorkerLifecycleRequest,
    ) -> Result<WorkerControllerSnapshot> {
        unreachable!("dispatch tests only ensure worker readiness")
    }

    async fn worker_status(
        &self,
        _request: WorkerStatusRequest,
    ) -> Result<Option<WorkerControllerSnapshot>> {
        unreachable!("dispatch tests only ensure worker readiness")
    }
}

fn sleeping_worker() -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot {
        id: WorkerName::new("worker-api")?,
        runtime: Some("copaw".to_string()),
        deployment: Some("remote".to_string()),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Sleeping,
        desired_state: Some("Stopped".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@worker-api:example.test".to_string()),
        room_id: Some("!stale-worker-room:example.test".to_string()),
        container_state: Some("stopped".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_000_000_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_000_010_000)),
        last_heartbeat_at: Some("2026-07-02T10:01:40Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill("api-implementation")],
        updated_at_ms: TimestampMs(1_783_000_010_000),
    })
}

fn local_worker() -> Result<NewAgentWorkerSnapshot> {
    Ok(NewAgentWorkerSnapshot { matrix_user_id: None, room_id: None, ..sleeping_worker()? })
}
