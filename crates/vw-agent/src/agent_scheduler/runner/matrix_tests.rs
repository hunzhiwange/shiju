//! 测试 Matrix 运行器的任务投递、响应关联与错误映射。
//! 关联标识和超时夹具确保只接受当前任务回复，并将远端失败保留为明确结果。

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};
use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use tower::ServiceExt;
use vw_api_types::ReasoningEffortDto;
use vw_api_types::agent_scheduler::{
    AgentSchedulerMatrixDispatchEventDto, AgentSchedulerWorkerRunEventKindDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::id::TenantScope;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

use super::adapters::controller::{
    FakeControllerAdapter, WorkerControllerSnapshot, WorkerLifecycleState,
};
use super::domain::{AgentRunStatus, AgentWorkerStatus, RunId, TaskId, WorkerName};
use super::engine::dispatch::{DispatchEngine, DispatchMode};
use super::events::AgentEventKind;
use super::runner::gateway_executor::gateway_local_agent_stream_request;
use super::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentWorkspace,
    LocalAgentWorkspaceRecycleRequest, LocalAgentWorkspaceReleaseRequest,
    LocalAgentWorkspaceRequest,
};
use super::runner::matrix::{
    MatrixAgentDispatchEvent, MatrixAgentDispatchEventSource, MatrixAgentWorkerRuntime,
    MatrixAgentWorkerRuntimeConfig, MatrixWorkerRunEventApi, MatrixWorkerRunEventApiRequest,
    configured_matrix_worker_from_stored,
};
use super::state_machine::{AgentTaskKind, AgentTaskStatus};
use super::store::runs::ClaimTaskRunRequest;
use super::store::tasks::NewAgentTask;
use super::store::workers::{AgentCapabilityRecord, NewAgentWorkerSnapshot, StoredAgentWorker};
use super::store::{AgentSchedulerStorageContext, AgentSchedulerStore};

const MATRIX_E2E_ROOM_ID: &str = "!room:matrix.example.com";
const MATRIX_E2E_USER_ID: &str = "@matrix-dev:matrix.example.com";
const MATRIX_E2E_WORKER_ID: &str = "matrix-dev";
const MATRIX_E2E_SKILL: &str = "matrix-e2e";

#[test]
fn matrix_worker_classification_trims_runtime_and_requires_exact_running_state() -> Result<()> {
    let mut worker = stored_matrix_worker()?;
    worker.runtime = Some("  MaTrIx  ".to_string());
    worker.desired_state = Some("  Running  ".to_string());

    assert!(super::runner::matrix::matrix_worker_runtime_is_matrix(&worker));
    assert!(super::runner::matrix::matrix_worker_desired_state_is_running(&worker));

    worker.runtime = Some("local".to_string());
    worker.desired_state = Some("running".to_string());
    assert!(!super::runner::matrix::matrix_worker_runtime_is_matrix(&worker));
    assert!(!super::runner::matrix::matrix_worker_desired_state_is_running(&worker));
    Ok(())
}

#[test]
fn configured_matrix_worker_trims_required_fields_and_rejects_missing_values() -> Result<()> {
    let mut worker = stored_matrix_worker()?;
    worker.matrix_user_id = Some("  @matrix-dev:matrix.example.com  ".to_string());
    worker.room_id = Some("  !room:matrix.example.com  ".to_string());

    let component = configured_matrix_worker_from_stored(Path::new("/repo/project"), worker)?;
    assert_eq!(component.matrix_user_id, "@matrix-dev:matrix.example.com");
    assert_eq!(component.room_id, "!room:matrix.example.com");

    let mut missing_user = stored_matrix_worker()?;
    missing_user.matrix_user_id = Some("   ".to_string());
    let error = configured_matrix_worker_from_stored(Path::new("/repo/project"), missing_user)
        .expect_err("blank Matrix user id should be rejected");
    assert_eq!(error.to_string(), "worker matrix-dev requires non-empty matrix_user_id");
    Ok(())
}

#[test]
fn matrix_runtime_config_validation_reports_the_invalid_field() -> Result<()> {
    let mut config = MatrixAgentWorkerRuntimeConfig {
        platform_id: "platform-a".to_string(),
        company_id: "company-a".to_string(),
        project_id: "project-a".to_string(),
        project_path: PathBuf::from("/repo/project"),
        worker_id: WorkerName::new("matrix-dev")?,
        matrix_user_id: MATRIX_E2E_USER_ID.to_string(),
        room_id: MATRIX_E2E_ROOM_ID.to_string(),
    };
    let api = RecordingWorkerRunEventApi::default();
    MatrixAgentWorkerRuntime::new(config.clone(), &api)?;

    config.company_id = "  ".to_string();
    let error = match MatrixAgentWorkerRuntime::new(config, &api) {
        Ok(_) => panic!("blank company id should be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "company_id must not be empty");
    Ok(())
}

#[tokio::test]
async fn matrix_worker_rejects_cross_scope_dispatch() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let mut runtime = runtime(&api)?;
    let mut wrong_platform = dispatch_event("$wrong-platform:matrix.example.com", "task-platform");
    wrong_platform.dispatch.platform_id = "platform-b".to_string();
    let mut wrong_company = dispatch_event("$wrong-company:matrix.example.com", "task-company");
    wrong_company.dispatch.company_id = "company-b".to_string();
    let owned = dispatch_event("$owned-scope:matrix.example.com", "task-owned-scope");
    let mut source = FakeMatrixDispatchEventSource::new(vec![wrong_platform, wrong_company, owned]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 1);
    assert_eq!(summary.ignored, 2);
    assert_eq!(api.requests().len(), 1);
    assert_eq!(api.requests()[0].platform_id, "platform-a");
    assert_eq!(api.requests()[0].company_id, "company-a");
    Ok(())
}

#[tokio::test]
async fn matrix_worker_accepts_only_assigned_dispatch_event() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let mut runtime = runtime(&api)?;
    let mut wrong_worker = dispatch_event("$wrong-worker:matrix.example.com", "task-wrong-worker");
    wrong_worker.dispatch.worker_id = "matrix-test".to_string();
    let mut wrong_user = dispatch_event("$wrong-user:matrix.example.com", "task-wrong-user");
    wrong_user.dispatch.matrix_user_id = "@matrix-test:matrix.example.com".to_string();
    let mut wrong_room = dispatch_event("$wrong-room:matrix.example.com", "task-wrong-room");
    wrong_room.room_id = "!other:matrix.example.com".to_string();
    wrong_room.dispatch.room_id = "!other:matrix.example.com".to_string();
    let owned = dispatch_event("$owned:matrix.example.com", "task-owned");
    let mut source =
        FakeMatrixDispatchEventSource::new(vec![wrong_worker, wrong_user, wrong_room, owned]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 1);
    assert_eq!(summary.ignored, 3);
    assert_eq!(api.requests().len(), 1);
    assert_eq!(api.requests()[0].task_id, "task-owned");
    Ok(())
}

#[tokio::test]
async fn matrix_worker_rejects_project_path_alias_dispatch() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let mut runtime = MatrixAgentWorkerRuntime::new(
        MatrixAgentWorkerRuntimeConfig {
            platform_id: "platform-a".to_string(),
            company_id: "company-a".to_string(),
            project_id: "stable-project-id".to_string(),
            project_path: PathBuf::from("/repo/project-1"),
            worker_id: WorkerName::new("matrix-dev")?,
            matrix_user_id: "@matrix-dev:matrix.example.com".to_string(),
            room_id: "!room:matrix.example.com".to_string(),
        },
        &api,
    )?;
    let mut wrong_project = dispatch_event("$wrong-path:matrix.example.com", "task-wrong-path");
    wrong_project.dispatch.project_id = "/repo/project-2".to_string();
    let mut event = dispatch_event("$path:matrix.example.com", "task-path");
    event.dispatch.project_id = "/repo/project-1/".to_string();
    let mut source = FakeMatrixDispatchEventSource::new(vec![wrong_project, event]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 0);
    assert_eq!(summary.ignored, 2);
    assert!(api.requests().is_empty());
    Ok(())
}

#[tokio::test]
async fn matrix_worker_deduplicates_dispatch_event_id() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let mut runtime = runtime(&api)?;
    let event = dispatch_event("$dup:matrix.example.com", "task-dup");
    let mut source = FakeMatrixDispatchEventSource::new(vec![event.clone(), event]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 1);
    assert_eq!(summary.duplicates, 1);
    assert_eq!(api.requests().len(), 1);
    Ok(())
}

#[tokio::test]
async fn matrix_worker_reports_started_event_for_owned_run() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let mut runtime = runtime(&api)?;
    let mut source = FakeMatrixDispatchEventSource::new(vec![dispatch_event(
        "$start:matrix.example.com",
        "task-start",
    )]);

    let summary = runtime.run_until_idle(&mut source).await?;
    let requests = api.requests();

    assert_eq!(summary.started, 1);
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.project_id, "project-1");
    assert_eq!(request.task_id, "task-start");
    assert_eq!(request.run_id, "run-task-start");
    assert_eq!(request.body.worker_id, "matrix-dev");
    assert_eq!(request.body.event, AgentSchedulerWorkerRunEventKindDto::Started);
    assert_eq!(request.body.payload["source"], "matrix_dispatch");
    assert_eq!(request.body.payload["matrix_event_id"], "$start:matrix.example.com");
    assert_eq!(request.body.payload["room_id"], "!room:matrix.example.com");
    Ok(())
}

#[tokio::test]
async fn matrix_worker_acquires_task_worktree_before_chat_stream() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let executor = RecordingLocalAgentExecutor::succeeding("matrix execution completed");
    let mut runtime = runtime_with_executor(&api, &executor)?;
    let mut source = FakeMatrixDispatchEventSource::new(vec![dispatch_event(
        "$worktree:matrix.example.com",
        "task-worktree",
    )]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 1);
    let actions = executor.actions();
    let lease_index = action_index(&actions, "lease:task-worktree");
    let execute_index = action_index(&actions, "execute:task-worktree");
    assert!(lease_index < execute_index, "actions should lease before execute: {actions:?}");

    let lease_request = executor.workspace_requests()[0].clone();
    assert_eq!(lease_request.project_path, PathBuf::from("/repo/project-1"));
    assert_eq!(lease_request.task_id.as_str(), "task-worktree");
    assert_eq!(lease_request.run_id.as_str(), "run-task-worktree");
    Ok(())
}

#[tokio::test]
async fn matrix_worker_uses_acp_chat_options_from_existing_executor() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let executor = RecordingLocalAgentExecutor::succeeding("matrix execution completed");
    let mut runtime = runtime_with_executor(&api, &executor)?;
    let mut event = dispatch_event("$acp:matrix.example.com", "task-acp");
    event.dispatch.model = "deepseek/deepseek-v4-flash".to_string();
    event.dispatch.agent = Some("reviewer".to_string());
    event.dispatch.acp_agent = Some("codex".to_string());
    let mut source = FakeMatrixDispatchEventSource::new(vec![event]);

    runtime.run_until_idle(&mut source).await?;

    let execution_request = executor.execution_requests()[0].clone();
    assert_eq!(execution_request.project_path, PathBuf::from("/repo/project-1"));
    assert_eq!(execution_request.model.as_deref(), Some("deepseek/deepseek-v4-flash"));
    assert_eq!(execution_request.agent.as_deref(), Some("reviewer"));
    assert_eq!(execution_request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(execution_request.workspace.path, "/repo/project-1/.vw/worktrees/matrix-slot");
    assert!(execution_request.prompt.contains("shared/tasks/task-acp/spec.md"));

    let stream_request = gateway_local_agent_stream_request(&execution_request);
    assert_eq!(stream_request.model.as_deref(), Some("deepseek/deepseek-v4-flash"));
    assert_eq!(stream_request.agent.as_deref(), Some("reviewer"));
    assert_eq!(stream_request.acp_agent.as_deref(), Some("codex"));
    let options = stream_request.options.expect("gateway options");
    assert_eq!(options["cwd"], "/repo/project-1/.vw/worktrees/matrix-slot");
    assert_eq!(options["full_access"], true);
    assert_eq!(options["agent"], "reviewer");
    assert_eq!(options["acp_agent"], "codex");
    assert_eq!(options["acp_permission_mode"], "approve-all");
    Ok(())
}

#[tokio::test]
async fn task_reasoning_effort_is_preserved_across_local_matrix_and_gateway_execution() -> Result<()>
{
    let api = RecordingWorkerRunEventApi::default();
    let executor = RecordingLocalAgentExecutor::succeeding("matrix execution completed");
    let mut runtime = runtime_with_executor(&api, &executor)?;
    let mut event = dispatch_event("$reasoning:matrix.example.com", "task-reasoning");
    event.dispatch.acp_agent = Some("codex".to_string());
    event.dispatch.reasoning_effort = Some(ReasoningEffortDto::High);
    let mut source = FakeMatrixDispatchEventSource::new(vec![event]);

    runtime.run_until_idle(&mut source).await?;

    let matrix_request = executor.execution_requests()[0].clone();
    assert_eq!(matrix_request.reasoning_effort, Some(ReasoningEffortDto::High));
    let matrix_gateway = gateway_local_agent_stream_request(&matrix_request);
    assert_eq!(matrix_gateway.reasoning_effort, Some(ReasoningEffortDto::High));

    let mut local_request = matrix_request;
    local_request.reasoning_effort = Some(ReasoningEffortDto::High);
    let local_gateway = gateway_local_agent_stream_request(&local_request);
    assert_eq!(local_gateway.reasoning_effort, matrix_gateway.reasoning_effort);

    local_request.acp_agent = None;
    let non_acp_gateway = gateway_local_agent_stream_request(&local_request);
    assert_eq!(non_acp_gateway.reasoning_effort, None);
    Ok(())
}

#[tokio::test]
async fn matrix_worker_releases_worktree_after_completion() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let executor = RecordingLocalAgentExecutor::succeeding("matrix execution completed");
    let mut runtime = runtime_with_executor(&api, &executor)?;
    let mut source = FakeMatrixDispatchEventSource::new(vec![dispatch_event(
        "$release:matrix.example.com",
        "task-release",
    )]);

    runtime.run_until_idle(&mut source).await?;

    let actions = executor.actions();
    let execute_index = action_index(&actions, "execute:task-release");
    let release_index = action_index(&actions, "release:task-release");
    assert!(execute_index < release_index, "actions should release after execute: {actions:?}");
    assert_eq!(executor.release_requests().len(), 1);
    assert!(executor.recycle_requests().is_empty());

    let events: Vec<_> = api.requests().into_iter().map(|request| request.body.event).collect();
    assert_eq!(
        events,
        vec![
            AgentSchedulerWorkerRunEventKindDto::Started,
            AgentSchedulerWorkerRunEventKindDto::Heartbeat,
            AgentSchedulerWorkerRunEventKindDto::Submitted,
            AgentSchedulerWorkerRunEventKindDto::Completed,
        ]
    );
    Ok(())
}

#[tokio::test]
async fn matrix_scheduler_e2e_dispatches_executes_and_completes_task() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-matrix-e2e");
    std::fs::create_dir_all(&project_path)?;
    let store = AgentSchedulerStore::new();
    let controller = FakeControllerAdapter::new();
    let task_id = TaskId::new("task-matrix-e2e")?;
    let worker_id = WorkerName::new(MATRIX_E2E_WORKER_ID)?;
    let now_ms = TimestampMs(1_783_200_000_000);

    store.initialize_project(&project_path)?;
    save_matrix_test_workflow(&store, &project_path)?;
    store.create_task(&project_path, matrix_e2e_task(&task_id))?;
    store.upsert_worker_snapshot(
        &project_path,
        matrix_worker_snapshot(&worker_id, AgentWorkerStatus::Idle, Some("Running")),
    )?;
    controller.set_snapshot(WorkerControllerSnapshot {
        worker_name: worker_id.clone(),
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("matrix".to_string()),
        team_id: None,
        worker_role: Some("worker".to_string()),
        room_id: Some(MATRIX_E2E_ROOM_ID.to_string()),
        container_state: Some("running".to_string()),
    });

    let dispatch = DispatchEngine::new(&store, &controller)
        .with_dispatch_mode(DispatchMode::Matrix)
        .dispatch_once(&project_path, now_ms)
        .await?
        .expect("queued task dispatches to Matrix worker");
    let outbox = store.claim_outbox_batch(&project_path, TimestampMs(now_ms.0 + 1), 10)?;
    assert_eq!(outbox.len(), 1);
    let dispatch_payload: AgentSchedulerMatrixDispatchEventDto =
        serde_json::from_value(outbox[0].payload.clone())?;
    assert_eq!(dispatch_payload.worker_id, MATRIX_E2E_WORKER_ID);
    assert_eq!(dispatch_payload.matrix_user_id, MATRIX_E2E_USER_ID);
    assert_eq!(dispatch_payload.room_id, MATRIX_E2E_ROOM_ID);
    assert_eq!(dispatch_payload.reasoning_effort, Some(ReasoningEffortDto::Max));

    let api = GatewayWorkerRunEventApi::default();
    let executor = RecordingLocalAgentExecutor::succeeding("matrix e2e completed");
    let mut runtime = MatrixAgentWorkerRuntime::new_with_executor(
        MatrixAgentWorkerRuntimeConfig {
            platform_id: dispatch_payload.platform_id.clone(),
            company_id: dispatch_payload.company_id.clone(),
            project_id: dispatch_payload.project_id.clone(),
            project_path: project_path.clone(),
            worker_id: worker_id.clone(),
            matrix_user_id: MATRIX_E2E_USER_ID.to_string(),
            room_id: MATRIX_E2E_ROOM_ID.to_string(),
        },
        &api,
        &executor,
    )?;
    let mut source = FakeMatrixDispatchEventSource::new(vec![MatrixAgentDispatchEvent {
        matrix_event_id: "$matrix-e2e:matrix.example.com".to_string(),
        room_id: MATRIX_E2E_ROOM_ID.to_string(),
        dispatch: dispatch_payload,
    }]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 1);
    assert_eq!(summary.ignored, 0);
    assert_eq!(summary.duplicates, 0);
    assert_eq!(executor.execution_requests()[0].reasoning_effort, Some(ReasoningEffortDto::Max));
    let task = store.load_task(&project_path, &task_id)?.expect("completed task is stored");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    let run = store.load_run(&project_path, &dispatch.run_id)?.expect("run is stored");
    assert_eq!(run.status, AgentRunStatus::Succeeded);
    let workspace = store
        .load_run_workspace_context(&project_path, &dispatch.run_id)?
        .expect("workspace is stored");
    assert_ne!(Path::new(&workspace.workspace.path), project_path.as_path());
    assert!(workspace.workspace.path.ends_with(".vw/worktrees/matrix-slot"));

    let request_events =
        api.requests().into_iter().map(|request| request.body.event).collect::<Vec<_>>();
    assert_eq!(
        request_events,
        vec![
            AgentSchedulerWorkerRunEventKindDto::Started,
            AgentSchedulerWorkerRunEventKindDto::Heartbeat,
            AgentSchedulerWorkerRunEventKindDto::Submitted,
            AgentSchedulerWorkerRunEventKindDto::Completed,
        ]
    );
    let event_kinds = store
        .list_events_after(&project_path, None, 20)?
        .into_iter()
        .map(|event| event.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        event_kinds,
        vec![
            AgentEventKind::WorkerSelected,
            AgentEventKind::MatrixDispatched,
            AgentEventKind::WorkerAcknowledged,
            AgentEventKind::RunStarted,
            AgentEventKind::RunSubmitted,
            AgentEventKind::RunCompleted,
        ]
    );
    let artifacts = store.list_task_artifacts(&project_path, &task_id)?;
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].kind, "execution_result");
    assert_eq!(artifacts[0].payload["success"], true);
    assert_eq!(artifacts[0].payload["output"], "matrix e2e completed");
    Ok(())
}

#[tokio::test]
async fn matrix_scheduler_rejects_event_for_wrong_worker() -> Result<()> {
    let api = RecordingWorkerRunEventApi::default();
    let mut runtime = runtime(&api)?;
    let mut wrong_worker = dispatch_event("$wrong-worker:matrix.example.com", "task-wrong-worker");
    wrong_worker.dispatch.worker_id = "matrix-test".to_string();
    let mut wrong_user = dispatch_event("$wrong-user:matrix.example.com", "task-wrong-user");
    wrong_user.dispatch.matrix_user_id = "@matrix-test:matrix.example.com".to_string();
    let mut wrong_room = dispatch_event("$wrong-room:matrix.example.com", "task-wrong-room");
    wrong_room.room_id = "!other:matrix.example.com".to_string();
    wrong_room.dispatch.room_id = "!other:matrix.example.com".to_string();
    let owned = dispatch_event("$owned:matrix.example.com", "task-owned");
    let mut source = FakeMatrixDispatchEventSource::new(vec![
        wrong_worker,
        wrong_user,
        wrong_room,
        owned.clone(),
        owned,
    ]);

    let summary = runtime.run_until_idle(&mut source).await?;

    assert_eq!(summary.started, 1);
    assert_eq!(summary.ignored, 3);
    assert_eq!(summary.duplicates, 1);
    assert_eq!(api.requests().len(), 1);

    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    let disabled_worker_id = WorkerName::new("matrix-disabled")?;
    store.initialize_project(project_path)?;
    let disabled_worker = store.upsert_worker_snapshot(
        project_path,
        matrix_worker_snapshot(
            &disabled_worker_id,
            AgentWorkerStatus::Unavailable,
            Some("Stopped"),
        ),
    )?;
    let error = configured_matrix_worker_from_stored(project_path, disabled_worker)
        .expect_err("disabled Matrix worker should not start");
    assert!(error.to_string().contains("desired_state must be Running"));
    Ok(())
}

#[tokio::test]
async fn matrix_scheduler_does_not_log_matrix_access_token() -> Result<()> {
    let secret = "matrix-secret-token-123456";
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().join("project-matrix-token-safety");
    std::fs::create_dir_all(&project_path)?;
    let scope = project_path.storage_scope()?;
    let project_scope = scope.project_scope();
    let platform_id = project_scope.platform_id().as_ref().to_string();
    let company_id = project_scope.company_id().as_ref().to_string();
    let project_id = project_scope.project_id().as_ref().to_string();
    let store = AgentSchedulerStore::new();
    let task_id = TaskId::new("task-matrix-token-safety")?;
    let run_id = RunId::new("run-task-matrix-token-safety")?;
    let worker_id = WorkerName::new(MATRIX_E2E_WORKER_ID)?;
    let now_ms = TimestampMs(1_783_201_000_000);

    store.initialize_project(&project_path)?;
    save_matrix_test_workflow(&store, &project_path)?;
    store.create_task(
        &project_path,
        NewAgentTask {
            id: task_id.clone(),
            title: "Keep Matrix access token out of scheduler records".to_string(),
            kind: AgentTaskKind::Finite,
            status: AgentTaskStatus::Dispatched,
            spec_uri: "shared/tasks/task-matrix-token-safety/spec.md".to_string(),
            meta_uri: "shared/tasks/task-matrix-token-safety/meta.json".to_string(),
            assigned_to: Some(worker_id.clone()),
            team_id: None,
            room_id: Some(MATRIX_E2E_ROOM_ID.to_string()),
            project_room_id: None,
            delegated_to_team: None,
            schedule: None,
            timezone: None,
            next_scheduled_at: None,
            last_executed_at: None,
            dag: Vec::new(),
            metadata: json!({"preferred_runtime": "matrix"}),
            created_at_ms: now_ms,
        },
    )?;
    store
        .claim_task_run(
            &project_path,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                lease_acquired_at_ms: now_ms,
                lease_expires_at_ms: Some(TimestampMs(now_ms.0 + 30_000)),
            },
        )?
        .expect("run is claimed");

    let api = GatewayWorkerRunEventApi::default();
    let executor = RecordingLocalAgentExecutor::failing(&format!(
        "Matrix request failed access_token={secret}"
    ));
    let mut runtime = MatrixAgentWorkerRuntime::new_with_executor(
        MatrixAgentWorkerRuntimeConfig {
            platform_id: platform_id.clone(),
            company_id: company_id.clone(),
            project_id: project_id.clone(),
            project_path: project_path.clone(),
            worker_id,
            matrix_user_id: MATRIX_E2E_USER_ID.to_string(),
            room_id: MATRIX_E2E_ROOM_ID.to_string(),
        },
        &api,
        &executor,
    )?;
    let event = MatrixAgentDispatchEvent {
        matrix_event_id: "$token-safety:matrix.example.com".to_string(),
        room_id: MATRIX_E2E_ROOM_ID.to_string(),
        dispatch: AgentSchedulerMatrixDispatchEventDto {
            event_version: 1,
            platform_id,
            company_id,
            project_id,
            task_id: task_id.as_str().to_string(),
            run_id: run_id.as_str().to_string(),
            worker_id: MATRIX_E2E_WORKER_ID.to_string(),
            worker_role: "worker".to_string(),
            room_id: MATRIX_E2E_ROOM_ID.to_string(),
            matrix_user_id: MATRIX_E2E_USER_ID.to_string(),
            model: "gpt-5".to_string(),
            model_provider: None,
            agent: None,
            acp_agent: None,
            reasoning_effort: None,
            spec_path: "shared/tasks/task-matrix-token-safety/spec.md".to_string(),
            metadata_path: "shared/tasks/task-matrix-token-safety/meta.json".to_string(),
            created_at_ms: now_ms,
        },
    };
    let dispatch_json = serde_json::to_string(&event.dispatch)?;
    assert!(!dispatch_json.contains(secret));
    let mut source = FakeMatrixDispatchEventSource::new(vec![event]);

    runtime.run_until_idle(&mut source).await?;

    let requests = api.requests();
    let artifacts = store.list_task_artifacts(&project_path, &task_id)?;
    let events = store.list_events_after(&project_path, None, 20)?;
    let logs = store.list_task_logs(&project_path, &task_id)?;
    let recycle_requests = executor.recycle_requests();
    assert_no_secret("worker event requests", &format!("{requests:?}"), secret);
    assert_no_secret("failed artifacts", &format!("{artifacts:?}"), secret);
    assert_no_secret("scheduler events", &format!("{events:?}"), secret);
    assert_no_secret("task logs", &format!("{logs:?}"), secret);
    assert_no_secret("recycle requests", &format!("{recycle_requests:?}"), secret);
    assert!(format!("{requests:?}").contains("[REDACTED]"));
    assert_eq!(artifacts[0].payload["success"], false);
    assert!(
        artifacts[0].payload["error"].as_str().is_some_and(|error| error.contains("[REDACTED]"))
    );
    Ok(())
}

fn runtime(
    api: &RecordingWorkerRunEventApi,
) -> Result<MatrixAgentWorkerRuntime<'_, RecordingWorkerRunEventApi>> {
    MatrixAgentWorkerRuntime::new(
        MatrixAgentWorkerRuntimeConfig {
            platform_id: "platform-a".to_string(),
            company_id: "company-a".to_string(),
            project_id: "project-1".to_string(),
            project_path: PathBuf::from("/repo/project-1"),
            worker_id: WorkerName::new("matrix-dev")?,
            matrix_user_id: "@matrix-dev:matrix.example.com".to_string(),
            room_id: "!room:matrix.example.com".to_string(),
        },
        api,
    )
}

fn runtime_with_executor<'a>(
    api: &'a RecordingWorkerRunEventApi,
    executor: &'a RecordingLocalAgentExecutor,
) -> Result<MatrixAgentWorkerRuntime<'a, RecordingWorkerRunEventApi, RecordingLocalAgentExecutor>> {
    MatrixAgentWorkerRuntime::new_with_executor(
        MatrixAgentWorkerRuntimeConfig {
            platform_id: "platform-a".to_string(),
            company_id: "company-a".to_string(),
            project_id: "project-1".to_string(),
            project_path: PathBuf::from("/repo/project-1"),
            worker_id: WorkerName::new("matrix-dev")?,
            matrix_user_id: "@matrix-dev:matrix.example.com".to_string(),
            room_id: "!room:matrix.example.com".to_string(),
        },
        api,
        executor,
    )
}

fn dispatch_event(matrix_event_id: &str, task_id: &str) -> MatrixAgentDispatchEvent {
    MatrixAgentDispatchEvent {
        matrix_event_id: matrix_event_id.to_string(),
        room_id: "!room:matrix.example.com".to_string(),
        dispatch: AgentSchedulerMatrixDispatchEventDto {
            event_version: 1,
            platform_id: "platform-a".to_string(),
            company_id: "company-a".to_string(),
            project_id: "project-1".to_string(),
            task_id: task_id.to_string(),
            run_id: format!("run-{task_id}"),
            worker_id: "matrix-dev".to_string(),
            worker_role: "worker".to_string(),
            room_id: "!room:matrix.example.com".to_string(),
            matrix_user_id: "@matrix-dev:matrix.example.com".to_string(),
            model: "gpt-5".to_string(),
            model_provider: None,
            agent: None,
            acp_agent: None,
            reasoning_effort: None,
            spec_path: format!("shared/tasks/{task_id}/spec.md"),
            metadata_path: format!("shared/tasks/{task_id}/meta.json"),
            created_at_ms: TimestampMs(1_000),
        },
    }
}

fn action_index(actions: &[String], expected: &str) -> usize {
    actions
        .iter()
        .position(|action| action == expected)
        .unwrap_or_else(|| panic!("missing action {expected}; got {actions:?}"))
}

struct FakeMatrixDispatchEventSource {
    events: VecDeque<MatrixAgentDispatchEvent>,
}

impl FakeMatrixDispatchEventSource {
    fn new(events: Vec<MatrixAgentDispatchEvent>) -> Self {
        Self { events: events.into() }
    }
}

#[async_trait]
impl MatrixAgentDispatchEventSource for FakeMatrixDispatchEventSource {
    async fn next_dispatch_event(&mut self) -> Result<Option<MatrixAgentDispatchEvent>> {
        Ok(self.events.pop_front())
    }
}

#[derive(Clone, Default)]
struct RecordingWorkerRunEventApi {
    requests: Arc<Mutex<Vec<MatrixWorkerRunEventApiRequest>>>,
}

impl RecordingWorkerRunEventApi {
    fn requests(&self) -> Vec<MatrixWorkerRunEventApiRequest> {
        self.requests.lock().expect("recording api lock").clone()
    }
}

#[async_trait]
impl MatrixWorkerRunEventApi for RecordingWorkerRunEventApi {
    async fn send_worker_run_event(&self, request: MatrixWorkerRunEventApiRequest) -> Result<()> {
        self.requests.lock().expect("recording api lock").push(request);
        Ok(())
    }
}

struct RecordingLocalAgentExecutor {
    output: Result<LocalAgentExecutionOutput, String>,
    actions: Mutex<Vec<String>>,
    workspace_requests: Mutex<Vec<LocalAgentWorkspaceRequest>>,
    execution_requests: Mutex<Vec<LocalAgentExecutionRequest>>,
    release_requests: Mutex<Vec<LocalAgentWorkspaceReleaseRequest>>,
    recycle_requests: Mutex<Vec<LocalAgentWorkspaceRecycleRequest>>,
}

impl RecordingLocalAgentExecutor {
    fn succeeding(output: &str) -> Self {
        Self {
            output: Ok(LocalAgentExecutionOutput { raw_output: output.to_string() }),
            actions: Mutex::new(Vec::new()),
            workspace_requests: Mutex::new(Vec::new()),
            execution_requests: Mutex::new(Vec::new()),
            release_requests: Mutex::new(Vec::new()),
            recycle_requests: Mutex::new(Vec::new()),
        }
    }

    fn failing(error: &str) -> Self {
        Self {
            output: Err(error.to_string()),
            actions: Mutex::new(Vec::new()),
            workspace_requests: Mutex::new(Vec::new()),
            execution_requests: Mutex::new(Vec::new()),
            release_requests: Mutex::new(Vec::new()),
            recycle_requests: Mutex::new(Vec::new()),
        }
    }

    fn actions(&self) -> Vec<String> {
        self.actions.lock().expect("actions lock").clone()
    }

    fn workspace_requests(&self) -> Vec<LocalAgentWorkspaceRequest> {
        self.workspace_requests.lock().expect("workspace requests lock").clone()
    }

    fn execution_requests(&self) -> Vec<LocalAgentExecutionRequest> {
        self.execution_requests.lock().expect("execution requests lock").clone()
    }

    fn release_requests(&self) -> Vec<LocalAgentWorkspaceReleaseRequest> {
        self.release_requests.lock().expect("release requests lock").clone()
    }

    fn recycle_requests(&self) -> Vec<LocalAgentWorkspaceRecycleRequest> {
        self.recycle_requests.lock().expect("recycle requests lock").clone()
    }
}

impl LocalAgentExecutor for RecordingLocalAgentExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        self.actions
            .lock()
            .expect("actions lock")
            .push(format!("lease:{}", request.task_id.as_str()));
        self.workspace_requests.lock().expect("workspace requests lock").push(request.clone());
        Ok(LocalAgentWorkspace {
            slot_id: "matrix-slot".to_string(),
            path: "/repo/project-1/.vw/worktrees/matrix-slot".to_string(),
            branch: format!("vw/task/{}", request.task_id.as_str()),
        })
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        self.actions
            .lock()
            .expect("actions lock")
            .push(format!("execute:{}", request.task_id.as_str()));
        self.execution_requests.lock().expect("execution requests lock").push(request);
        self.output.clone().map_err(anyhow::Error::msg)
    }

    fn release_workspace(&self, request: LocalAgentWorkspaceReleaseRequest) -> Result<()> {
        self.actions
            .lock()
            .expect("actions lock")
            .push(format!("release:{}", request.task_id.as_str()));
        self.release_requests.lock().expect("release requests lock").push(request);
        Ok(())
    }

    fn recycle_workspace(&self, request: LocalAgentWorkspaceRecycleRequest) -> Result<()> {
        self.actions
            .lock()
            .expect("actions lock")
            .push(format!("recycle:{}", request.task_id.as_str()));
        self.recycle_requests.lock().expect("recycle requests lock").push(request);
        Ok(())
    }
}

#[derive(Default)]
struct GatewayWorkerRunEventApi {
    requests: Arc<Mutex<Vec<MatrixWorkerRunEventApiRequest>>>,
}

impl GatewayWorkerRunEventApi {
    fn requests(&self) -> Vec<MatrixWorkerRunEventApiRequest> {
        self.requests.lock().expect("gateway api requests lock").clone()
    }
}

#[async_trait]
impl MatrixWorkerRunEventApi for GatewayWorkerRunEventApi {
    async fn send_worker_run_event(&self, request: MatrixWorkerRunEventApiRequest) -> Result<()> {
        self.requests.lock().expect("gateway api requests lock").push(request.clone());
        let body = serde_json::to_vec(&request.body)?;
        let uri = format!(
            "/agent-scheduler/projects/{}/tasks/{}/runs/{}/worker-events",
            urlencoding::encode(&request.project_id),
            urlencoding::encode(&request.task_id),
            urlencoding::encode(&request.run_id),
        );
        let tenant = TenantScope::new(request.platform_id.clone(), request.company_id.clone())?;
        let app = crate::gateway::api::handlers::agent_scheduler::router::<()>()
            .layer(axum::Extension(tenant))
            .with_state(());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))?,
            )
            .await?;
        if response.status() != StatusCode::OK {
            let status = response.status();
            let body = to_bytes(response.into_body(), usize::MAX).await?;
            bail!(
                "Matrix worker event gateway request failed with {status}: {}",
                String::from_utf8_lossy(&body)
            );
        }
        Ok(())
    }
}

fn matrix_e2e_task(task_id: &TaskId) -> NewAgentTask {
    NewAgentTask {
        id: task_id.clone(),
        title: "Run Matrix scheduler E2E".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: format!("shared/tasks/{}/spec.md", task_id.as_str()),
        meta_uri: format!("shared/tasks/{}/meta.json", task_id.as_str()),
        assigned_to: None,
        team_id: None,
        room_id: Some(MATRIX_E2E_ROOM_ID.to_string()),
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "required_skills": [MATRIX_E2E_SKILL],
            "preferred_runtime": "matrix",
            "agent": "codex",
            "acp_agent": "claude",
            "reasoning_effort": "max",
        }),
        created_at_ms: TimestampMs(1_783_199_900_000),
    }
}

fn save_matrix_test_workflow(store: &AgentSchedulerStore, project_path: &Path) -> Result<()> {
    let status = |key: &str,
                  role: TaskPoolWorkflowStatusRoleDto,
                  transitions: Vec<TaskPoolWorkflowTransitionDto>| {
        TaskPoolWorkflowStatusDto {
            key: key.to_string(),
            label: key.to_string(),
            role,
            enabled: true,
            terminal: false,
            next: None,
            transitions,
        }
    };
    let transition = |outcome: &str, target: &str| TaskPoolWorkflowTransitionDto {
        outcome: outcome.to_string(),
        label: None,
        target: target.to_string(),
    };
    store.save_task_board_workflow(
        project_path,
        &TaskPoolWorkflowSnapshotDto {
            id: "matrix_test_flow".to_string(),
            label: "Matrix Test Flow".to_string(),
            statuses: vec![
                status("pending", TaskPoolWorkflowStatusRoleDto::Queued, Vec::new()),
                status(
                    "running",
                    TaskPoolWorkflowStatusRoleDto::Executing,
                    vec![transition("success", "code_complete"), transition("failure", "failed")],
                ),
                status(
                    "code_complete",
                    TaskPoolWorkflowStatusRoleDto::ReviewReady,
                    vec![transition("success", "done")],
                ),
                status("failed", TaskPoolWorkflowStatusRoleDto::Failed, Vec::new()),
                status("done", TaskPoolWorkflowStatusRoleDto::Done, Vec::new()),
            ],
        },
    )?;
    Ok(())
}

fn matrix_worker_snapshot(
    worker_id: &WorkerName,
    status: AgentWorkerStatus,
    desired_state: Option<&str>,
) -> NewAgentWorkerSnapshot {
    NewAgentWorkerSnapshot {
        id: worker_id.clone(),
        runtime: Some("matrix".to_string()),
        deployment: Some("local".to_string()),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        status,
        desired_state: desired_state.map(str::to_string),
        team_id: None,
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some(format!("@{}:matrix.example.com", worker_id.as_str())),
        room_id: Some(MATRIX_E2E_ROOM_ID.to_string()),
        container_state: Some("running".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_199_800_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_199_850_000)),
        last_heartbeat_at: Some("2026-07-05T00:00:00Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill(MATRIX_E2E_SKILL)],
        updated_at_ms: TimestampMs(1_783_199_850_000),
    }
}

fn stored_matrix_worker() -> Result<StoredAgentWorker> {
    Ok(StoredAgentWorker {
        id: WorkerName::new(MATRIX_E2E_WORKER_ID)?,
        runtime: Some("matrix".to_string()),
        deployment: Some("local".to_string()),
        model: "gpt-5-mini".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatus::Idle,
        desired_state: Some("Running".to_string()),
        team_id: None,
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some(MATRIX_E2E_USER_ID.to_string()),
        room_id: Some(MATRIX_E2E_ROOM_ID.to_string()),
        container_state: Some("running".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_199_800_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_199_850_000)),
        last_heartbeat_at: Some("2026-07-05T00:00:00Z".to_string()),
        capabilities: vec![AgentCapabilityRecord::skill(MATRIX_E2E_SKILL)],
        updated_at_ms: TimestampMs(1_783_199_850_000),
    })
}

fn assert_no_secret(label: &str, haystack: &str, secret: &str) {
    assert!(
        !haystack.contains(secret),
        "{label} should not contain raw Matrix access token: {haystack}"
    );
}
