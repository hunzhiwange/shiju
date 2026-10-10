//! 测试调度引擎分派就绪任务时的选员、租约与失败回滚。
//! 竞争领取和执行失败夹具确认任务只分派一次，失败后状态与租约同步收敛。

use anyhow::Result;
use serde_json::json;
use vw_api_types::ReasoningEffortDto;
use vw_api_types::agent_scheduler::AgentSchedulerMatrixDispatchEventDto;
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
use crate::agent_scheduler::adapters::matrix::MatrixDispatchTarget;
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
async fn agent_dispatch_engine_claims_task_and_enqueues_worker_message() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_300_000);

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

    let engine = DispatchEngine::new(&store, &controller);
    let outcome =
        engine.dispatch_once(project_path, now_ms).await?.expect("pending task dispatches");

    assert_eq!(outcome.task_id.as_str(), "task-dispatch-1");
    assert_eq!(outcome.worker_id.as_str(), "worker-api");
    let DispatchTarget::Matrix(target) = &outcome.target else {
        panic!("Matrix worker should dispatch through Matrix target");
    };
    assert_eq!(target.room_id, "!worker-api-room:example.test");
    assert_eq!(target.recipient_id, "@worker-api:example.test");

    let ready_requests = controller.recorded_lifecycle_requests();
    assert_eq!(ready_requests.len(), 1);
    assert_eq!(ready_requests[0].worker_name.as_str(), "worker-api");
    assert_eq!(ready_requests[0].desired_state, WorkerLifecycleState::Running);

    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    assert_eq!(task.status, AgentTaskStatus::Dispatched);

    let stale_at_expiry = store
        .list_stale_runs(project_path, TimestampMs(now_ms.0 + DEFAULT_RUN_LEASE_DURATION_MS))?;
    assert_eq!(stale_at_expiry.len(), 1);
    assert_eq!(stale_at_expiry[0].id, outcome.run_id);
    assert_eq!(stale_at_expiry[0].worker_id.as_ref().map(WorkerName::as_str), Some("worker-api"),);

    let events = store.list_events_after(project_path, None, 10)?;
    let kinds = events.iter().map(|event| event.kind).collect::<Vec<_>>();
    assert_eq!(kinds, vec![AgentEventKind::WorkerSelected, AgentEventKind::MatrixDispatched]);
    assert_eq!(events[0].payload["worker_id"], "worker-api");
    assert_eq!(events[0].payload["dispatch_target"], "matrix");
    let outbox_message_id =
        outcome.outbox_message_id.as_deref().expect("Matrix target writes outbox");
    assert_eq!(events[1].payload["outbox_message_id"], outbox_message_id);

    let outbox = store.claim_outbox_batch(project_path, TimestampMs(now_ms.0 + 1), 10)?;
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0].id, outbox_message_id);
    assert_eq!(outbox[0].channel, "matrix");
    assert_eq!(outbox[0].target, "room:!worker-api-room:example.test");
    assert_eq!(outbox[0].recipient_id.as_deref(), Some("@worker-api:example.test"));
    assert!(outbox[0].body.contains("@worker-api:example.test"));
    assert!(outbox[0].body.contains("[task-dispatch-1]"));
    assert!(outbox[0].body.contains("Implement dispatch engine"));
    assert!(outbox[0].body.contains("shared/tasks/task-dispatch-1/spec.md"));

    Ok(())
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
async fn dispatch_task_once_dispatches_named_queued_task_after_lifecycle_check() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_310_000);

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

    let outcome = DispatchEngine::new(&store, &controller)
        .dispatch_task_once(project_path, &TaskId::new("task-dispatch-1")?, now_ms)
        .await?
        .expect("named queued task dispatches");

    assert_eq!(outcome.task_id.as_str(), "task-dispatch-1");
    assert_eq!(outcome.worker_id.as_str(), "worker-api");
    assert!(matches!(outcome.target, DispatchTarget::Matrix(_)));
    assert!(outcome.outbox_message_id.is_some());
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
            assert_eq!(outcome.worker_id.as_str(), "worker-api");
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
async fn dispatch_outbox_payload_contains_worker_run_contract() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_300_000);

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

    let engine = DispatchEngine::new(&store, &controller);
    let outcome =
        engine.dispatch_once(project_path, now_ms).await?.expect("pending task dispatches");

    let outbox = store.claim_outbox_batch(project_path, TimestampMs(now_ms.0 + 1), 10)?;
    assert_eq!(outbox.len(), 1);
    let payload: AgentSchedulerMatrixDispatchEventDto =
        serde_json::from_value(outbox[0].payload.clone())?;

    assert_eq!(payload.event_version, 1);
    assert_eq!(
        payload.project_id,
        project_path.storage_scope()?.project_scope().project_id().as_ref()
    );
    assert_eq!(payload.task_id, "task-dispatch-1");
    assert_eq!(payload.run_id, outcome.run_id.as_str());
    assert_eq!(payload.worker_id, "worker-api");
    assert_eq!(payload.worker_role, "worker");
    assert_eq!(payload.room_id, "!worker-api-room:example.test");
    assert_eq!(payload.matrix_user_id, "@worker-api:example.test");
    assert_eq!(payload.model, "gpt-5-mini");
    assert_eq!(payload.model_provider.as_deref(), Some("openai"));
    assert_eq!(payload.agent.as_deref(), Some("codex"));
    assert_eq!(payload.acp_agent.as_deref(), Some("claude"));
    assert_eq!(payload.reasoning_effort, Some(ReasoningEffortDto::Max));
    assert_eq!(payload.spec_path, "shared/tasks/task-dispatch-1/spec.md");
    assert_eq!(payload.metadata_path, "shared/tasks/task-dispatch-1/meta.json");
    assert_eq!(payload.created_at_ms, now_ms);

    Ok(())
}

#[tokio::test]
async fn matrix_dispatch_preserves_task_reasoning_effort() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_300_000);

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

    DispatchEngine::new(&store, &controller)
        .dispatch_once(project_path, now_ms)
        .await?
        .expect("pending task dispatches");
    let outbox = store.claim_outbox_batch(project_path, TimestampMs(now_ms.0 + 1), 10)?;
    let payload: AgentSchedulerMatrixDispatchEventDto =
        serde_json::from_value(outbox[0].payload.clone())?;

    assert_eq!(payload.acp_agent.as_deref(), Some("claude"));
    assert_eq!(payload.reasoning_effort, Some(ReasoningEffortDto::Max));
    Ok(())
}

#[tokio::test]
async fn agent_dispatch_engine_defaults_to_local_target_without_matrix() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_400_000);

    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;
    store.upsert_worker_snapshot(project_path, local_worker()?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: None,
        container_state: Some("ready".to_string()),
    });

    let engine = DispatchEngine::new(&store, &controller);
    let outcome =
        engine.dispatch_once(project_path, now_ms).await?.expect("pending task dispatches");

    assert_eq!(outcome.task_id.as_str(), "task-dispatch-1");
    assert_eq!(outcome.worker_id.as_str(), "worker-api");
    assert!(outcome.outbox_message_id.is_none());
    let DispatchTarget::Local(target) = &outcome.target else {
        panic!("worker without Matrix address should dispatch through local target");
    };
    assert_eq!(target.task_id.as_str(), "task-dispatch-1");
    assert_eq!(target.run_id, outcome.run_id);
    assert_eq!(target.worker_id.as_str(), "worker-api");
    assert_eq!(target.requested_at_ms, now_ms);

    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    assert_eq!(task.status, AgentTaskStatus::Dispatched);

    let events = store.list_events_after(project_path, None, 10)?;
    let kinds = events.iter().map(|event| event.kind).collect::<Vec<_>>();
    assert_eq!(kinds, vec![AgentEventKind::WorkerSelected]);
    assert_eq!(events[0].payload["dispatch_target"], "local");

    let outbox = store.claim_outbox_batch(project_path, TimestampMs(now_ms.0 + 1), 10)?;
    assert!(outbox.is_empty());

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
async fn agent_dispatch_engine_requires_matrix_address_in_matrix_mode() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_600_000);

    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;
    store.upsert_worker_snapshot(project_path, local_worker()?)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: None,
        container_state: Some("ready".to_string()),
    });

    let engine = DispatchEngine::new(&store, &controller).with_dispatch_mode(DispatchMode::Matrix);
    let error = engine
        .dispatch_once(project_path, now_ms)
        .await
        .expect_err("Matrix mode requires Matrix worker address");

    assert!(error.to_string().contains("Matrix dispatch requires room_id"));
    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    assert_eq!(task.status, AgentTaskStatus::Queued);

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

#[tokio::test]
async fn agent_dispatch_engine_requires_matrix_user_in_matrix_mode() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let now_ms = TimestampMs(1_783_000_720_000);
    let mut worker = sleeping_worker()?;
    worker.matrix_user_id = None;

    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;
    store.upsert_worker_snapshot(project_path, worker)?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let error = DispatchEngine::new(&store, &controller)
        .with_dispatch_mode(DispatchMode::Matrix)
        .dispatch_once(project_path, now_ms)
        .await
        .expect_err("Matrix mode requires worker Matrix user id");

    assert!(error.to_string().contains("Matrix dispatch requires matrix_user_id"));
    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    Ok(())
}

#[test]
fn dispatch_helper_edges_cover_metadata_status_target_and_payload_defaults() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let now_ms = TimestampMs(1_783_000_730_000);
    store.initialize_project(project_path)?;
    store.create_task(project_path, pending_task()?)?;
    store.transition_task_status(
        project_path,
        &TaskId::new("task-dispatch-1")?,
        AgentTaskStatus::Assigned,
        now_ms,
    )?;
    store.upsert_worker_snapshot(project_path, sleeping_worker()?)?;
    let task =
        store.load_task(project_path, &TaskId::new("task-dispatch-1")?)?.expect("task stored");
    let worker =
        store.load_worker(project_path, &WorkerName::new("worker-api")?)?.expect("worker stored");
    let run = store
        .claim_task_run(
            project_path,
            ClaimTaskRunRequest {
                run_id: RunId::new("run-helper")?,
                task_id: task.id.clone(),
                worker_id: worker.id.clone(),
                lease_acquired_at_ms: now_ms,
                lease_expires_at_ms: None,
            },
        )?
        .expect("task is claimable");

    let mut mapped_status = task.clone();
    mapped_status.metadata = json!({
        "task_board.status_key": " ",
        "task_board.status_key_map": {
            "assigned": "custom_assigned"
        },
    });
    assert_eq!(super::dispatch::dispatch_status_key(&mapped_status), "custom_assigned");
    assert_eq!(
        super::dispatch::legacy_dispatch_status_key(AgentTaskStatus::Submitted),
        "code_complete"
    );
    assert_eq!(super::dispatch::legacy_dispatch_status_key(AgentTaskStatus::Running), "running");

    let mut fallback_metadata = task.clone();
    fallback_metadata.metadata = json!({
        "skills": ["docs"],
        "runtime": "  local  ",
        "model": "auto",
        "agent": " main ",
        "acp_agent": "default"
    });
    let selection = super::dispatch::selection_request_from_task(&fallback_metadata)?;
    assert_eq!(selection.required_skills, vec!["docs".to_string()]);
    assert_eq!(selection.preferred_runtime.as_deref(), Some("local"));
    assert_eq!(super::dispatch::dispatch_model(&fallback_metadata), None);
    assert_eq!(super::dispatch::dispatch_acp_agent(&fallback_metadata), None);
    assert_eq!(super::dispatch::dispatch_agent(&fallback_metadata, Some("claude")), None);

    let mut no_runtime_metadata = task.clone();
    no_runtime_metadata.metadata = json!({
        "skills": [],
        "model": "auto",
        "agent": "codex",
        "acp_agent": "internal",
    });
    let no_runtime = super::dispatch::selection_request_from_task(&no_runtime_metadata)?;
    assert_eq!(no_runtime.required_skills, Vec::<String>::new());
    assert_eq!(no_runtime.preferred_runtime, None);
    assert_eq!(super::dispatch::dispatch_acp_agent(&no_runtime_metadata), None);

    let mut blank_runtime_metadata = task.clone();
    blank_runtime_metadata.metadata = json!({
        "skills": ["docs"],
        "preferred_runtime": "   ",
    });
    let blank_runtime = super::dispatch::selection_request_from_task(&blank_runtime_metadata)?;
    assert_eq!(blank_runtime.preferred_runtime, None);

    let mut bad_skills_metadata = task.clone();
    bad_skills_metadata.metadata = json!({
        "required_skills": ["docs", 7],
    });
    let bad_skills = super::dispatch::selection_request_from_task(&bad_skills_metadata)
        .expect_err("required skills array must only contain strings");
    assert!(
        bad_skills.to_string().contains("task metadata field required_skills must contain strings")
    );

    let missing_candidate = WorkerCandidateScore {
        worker_id: WorkerName::new("worker-missing")?,
        availability: WorkerAvailability::Idle,
        finite_active_tasks: 0,
        infinite_active_tasks: 0,
        active_tasks: 0,
        runtime_matched: true,
        can_dispatch_now: true,
        requires_wakeup: false,
    };
    let missing_worker = super::dispatch::selected_worker(&[worker.clone()], &missing_candidate)
        .expect_err("candidate must reference a listed worker");
    assert!(missing_worker.to_string().contains("selected worker snapshot missing"));

    let mut worker_without_room = worker.clone();
    worker_without_room.room_id = None;
    let ready_without_room = WorkerControllerSnapshot {
        worker_name: worker.id.clone(),
        desired_state: WorkerLifecycleState::Running,
        runtime: worker.runtime.clone(),
        team_id: worker.team_id.clone(),
        worker_role: worker.worker_role.clone(),
        room_id: None,
        container_state: Some("ready".to_string()),
    };
    let matrix_room_error = super::dispatch::dispatch_target_from_worker(
        &ready_without_room,
        &worker_without_room,
        &task,
        &run,
        now_ms,
        DispatchMode::Matrix,
    )
    .expect_err("Matrix mode requires room id");
    assert!(matrix_room_error.to_string().contains("Matrix dispatch requires room_id"));

    let mut worker_without_user = worker.clone();
    worker_without_user.matrix_user_id = None;
    let ready_with_room = WorkerControllerSnapshot {
        room_id: Some("!room:example.test".to_string()),
        ..ready_without_room
    };
    let matrix_user_error = super::dispatch::dispatch_target_from_worker(
        &ready_with_room,
        &worker_without_user,
        &task,
        &run,
        now_ms,
        DispatchMode::Matrix,
    )
    .expect_err("Matrix mode requires matrix user id");
    assert!(matrix_user_error.to_string().contains("Matrix dispatch requires matrix_user_id"));
    let validation_error = super::dispatch::validate_dispatch_mode_available(
        &ready_with_room,
        &worker_without_user,
        DispatchMode::Matrix,
    )
    .expect_err("Matrix mode validates matrix user id before dispatch");
    assert!(validation_error.to_string().contains("Matrix dispatch requires matrix_user_id"));

    let matrix_target = super::dispatch::dispatch_target_from_worker(
        &ready_with_room,
        &worker,
        &task,
        &run,
        now_ms,
        DispatchMode::Matrix,
    )?;
    let DispatchTarget::Matrix(matrix_target) = matrix_target else {
        panic!("Matrix mode with room and user must resolve a Matrix target");
    };
    assert_eq!(matrix_target.room_id, "!room:example.test");
    assert_eq!(matrix_target.recipient_id, "@worker-api:example.test");
    super::dispatch::validate_dispatch_mode_available(
        &ready_with_room,
        &worker,
        DispatchMode::Matrix,
    )?;

    let mut no_acp_task = task.clone();
    no_acp_task.metadata = json!({
        "model": "gpt-5",
        "agent": "main"
    });
    let payload = super::dispatch::matrix_dispatch_event(
        project_path,
        &no_acp_task,
        &worker,
        &run,
        &MatrixDispatchTarget {
            room_id: "!room:example.test".to_string(),
            recipient_id: "@worker-api:example.test".to_string(),
        },
        now_ms,
    )?;
    assert_eq!(payload.model, "gpt-5");
    assert_eq!(payload.agent.as_deref(), Some("main"));
    assert_eq!(payload.acp_agent, None);
    assert_eq!(payload.reasoning_effort, None);

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
