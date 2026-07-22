use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;
use vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentSchedulerBulkCommandRequest,
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyTaskImportDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerStageActionCommandPayloadDto, AgentSchedulerStageActionInputsDto,
    AgentSchedulerStageActionOutcomeDto, AgentSchedulerTaskMetadataPatchDto,
    AgentSchedulerTaskMetadataUpdateRequest, AgentSchedulerUpsertTeamRequest,
    AgentSchedulerUpsertWorkerRequest, AgentSchedulerWorkerRunEventKindDto,
    AgentSchedulerWorkerRunEventRequest, AgentTaskKindDto, AgentTaskStatusDto, AgentTeamMemberDto,
    AgentTeamStatusDto, AgentWorkerStatusDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};
use vw_config_types::staff::{
    StaffCapabilityConfig, StaffCapabilityIoSummaryConfig, StaffCapabilityKindConfig, StaffConfig,
    StaffStageAssignmentConfig, StaffWorkflowOrchestrationConfig,
};

#[tokio::test]
async fn agent_scheduler_routes_require_header_tenant_scope() {
    let app = router::<()>()
        .layer(axum::middleware::from_fn(
            crate::app::agent::gateway::middleware::tenant_scope_middleware,
        ))
        .with_state(());
    let old = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/agent-scheduler/platforms/platform-a/companies/company-a/projects/project-a/tasks")
                .header(vw_api_types::id::PLATFORM_ID_HEADER, "platform-a")
                .header(vw_api_types::id::COMPANY_ID_HEADER, "company-a")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(old.status(), StatusCode::NOT_FOUND);

    let missing_tenant = app
        .oneshot(
            Request::builder()
                .uri("/agent-scheduler/projects/project-a/tasks")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(missing_tenant.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn agent_scheduler_sse_uses_extension_tenant_scope_and_isolates_projects() {
    let project_id = format!("scheduler-sse-scope-{}", Uuid::new_v4());
    let tenant_a =
        TenantScope::new("scheduler-platform-a", "scheduler-company-a").expect("tenant a");
    let tenant_b =
        TenantScope::new("scheduler-platform-b", "scheduler-company-b").expect("tenant b");
    let temp = tempfile::tempdir().expect("temp project root");
    let worktree_a = temp.path().join("tenant-a");
    let worktree_b = temp.path().join("tenant-b");
    std::fs::create_dir_all(&worktree_a).expect("tenant a worktree");
    std::fs::create_dir_all(&worktree_b).expect("tenant b worktree");
    let store = project::runtime_store().expect("project store");
    let project_a = project::Info {
        id: project_id.clone(),
        worktree: worktree_a.to_string_lossy().to_string(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    let project_b =
        project::Info { worktree: worktree_b.to_string_lossy().to_string(), ..project_a.clone() };
    store.create(&tenant_a, &project_a).expect("store tenant a project");
    store.create(&tenant_b, &project_b).expect("store tenant b project");

    let gateway = Arc::new(FakeAgentSchedulerGateway::default());
    for tenant in [&tenant_a, &tenant_b] {
        let response = router_with_gateway::<()>(gateway.clone())
            .layer(Extension(tenant.clone()))
            .with_state(())
            .oneshot(
                Request::builder()
                    .uri(format!("/agent-scheduler/projects/{project_id}/events/stream"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("SSE response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let contexts = gateway.state.lock().expect("fake scheduler state lock").scoped_contexts.clone();
    assert_eq!(contexts.len(), 2);
    assert_eq!(contexts[0].scope().project_scope().tenant_scope(), &tenant_a);
    assert_eq!(contexts[0].project_path(), worktree_a);
    assert_eq!(contexts[1].scope().project_scope().tenant_scope(), &tenant_b);
    assert_eq!(contexts[1].project_path(), worktree_b);

    for tenant in [&tenant_a, &tenant_b] {
        let scope = ProjectScope::new(tenant.clone(), ProjectId::from(project_id.as_str()))
            .expect("project scope");
        store.delete(&scope).expect("remove test project");
    }
}

#[tokio::test]
async fn agent_scheduler_api_creates_task_and_wakes_dispatch_engine() {
    let gateway = FakeAgentSchedulerGateway::default();
    let project_path = String::new();

    let created = agent_scheduler_create_task_with_gateway(
        &gateway,
        AgentSchedulerCreateTaskRequest {
            task_id: "task-api-1".to_string(),
            title: "Implement API route".to_string(),
            kind: AgentTaskKindDto::Finite,
            status: Some(AgentTaskStatusDto::Queued),
            spec_uri: "shared/tasks/task-api-1/spec.md".to_string(),
            meta_uri: "shared/tasks/task-api-1/meta.json".to_string(),
            assigned_to: None,
            team_id: Some("team-api".to_string()),
            room_id: None,
            project_room_id: Some("!project-room:example.test".to_string()),
            delegated_to_team: None,
            schedule: None,
            timezone: None,
            next_scheduled_at: None,
            last_executed_at: None,
            metadata: json!({"required_skills": ["api-route"]}),
            created_at_ms: TimestampMs(1_783_000_400_000),
        },
    )
    .await
    .expect("create task");

    assert_eq!(created.task_id, "task-api-1");

    let wake = agent_scheduler_wake_with_gateway(
        &gateway,
        AgentSchedulerWakeRequest {
            project_path: project_path.clone(),
            now_ms: TimestampMs(1_783_000_401_000),
            dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
            max_concurrent: None,
            auto_promote_pool_tasks: false,
        },
    )
    .await
    .expect("wake scheduler");

    assert_eq!(wake.task_id.as_deref(), Some("task-api-1"));
    assert_eq!(wake.outbox_message_id.as_deref(), Some("outbox-task-api-1"));

    let events = agent_scheduler_list_events_with_gateway(
        &gateway,
        AgentSchedulerEventsQuery {
            project_path: project_path.clone(),
            after_cursor: None,
            limit: None,
        },
    )
    .await
    .expect("list events");

    assert_eq!(events.cursor, 2);
    assert_eq!(events.events.len(), 2);

    let outbox = agent_scheduler_drain_outbox_with_gateway(
        &gateway,
        AgentSchedulerOutboxDrainRequest {
            project_path,
            now_ms: TimestampMs(1_783_000_402_000),
            limit: Some(10),
        },
    )
    .await
    .expect("drain outbox");

    assert_eq!(outbox.messages.len(), 1);
    assert_eq!(outbox.messages[0].id, "outbox-task-api-1");
}

#[tokio::test]
async fn agent_scheduler_gateway_command_runs_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-gateway");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-gateway-new").expect("task id"),
                title: "Run gateway task through agent scheduler".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-gateway-new/spec.md".to_string(),
                meta_uri: "shared/tasks/task-gateway-new/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Run this through the Agent Scheduler local runner",
                    "priority": 3,
                    "order_no": 7,
                    "model": "auto",
                    "agent": "main"
                }),
                created_at_ms: TimestampMs(1_783_000_500_000),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "mock local execution output".to_string(),
    });
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: "task-gateway-new".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("ready from gateway".to_string()),
        payload: Default::default(),
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["task"]["task_id"], "task-gateway-new");
    assert_ne!(body["task"]["status"], "queued");
    assert_eq!(body["event"]["payload"]["command"], "start");

    let task_id = TaskId::new("task-gateway-new").expect("task id");
    let task =
        store.load_task(&project_path, &task_id).expect("load scheduler task").expect("task");
    assert_ne!(task.status, AgentTaskStatus::Queued);

    let logs_uri = format!("/agent-scheduler/projects/{project_id}/tasks/task-gateway-new/logs");
    let logs = wait_for_log(app.clone(), &logs_uri, "local execution submitted result").await;
    assert_eq!(logs["task_id"], "task-gateway-new");
    assert!(
        logs["logs"]
            .as_array()
            .expect("logs array")
            .iter()
            .any(|log| log["message"] == "local execution submitted result")
    );
    let task =
        store.load_task(&project_path, &task_id).expect("load scheduler task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);

    let snapshot =
        get_json(app.clone(), &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert_eq!(snapshot["tasks"][0]["task_id"], "task-gateway-new");
    assert_eq!(snapshot["tasks"][0]["status"], "submitted");
    assert_eq!(snapshot["tasks"][0]["status_key"], "code_complete");

    let artifacts = get_json(
        app,
        &format!("/agent-scheduler/projects/{project_id}/tasks/task-gateway-new/artifacts"),
    )
    .await;
    assert_eq!(artifacts["task_id"], "task-gateway-new");
    assert_eq!(artifacts["artifacts"][0]["kind"], "execution_result");
    assert_eq!(artifacts["artifacts"][0]["payload"]["success"], true);
    assert_eq!(artifacts["artifacts"][0]["payload"]["output"], "mock local execution output");
}

#[tokio::test]
async fn agent_scheduler_gateway_local_command_respects_max_concurrent() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-command-concurrency");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "should not run behind command concurrency limit".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");

    for (task_id, status) in [
        ("task-command-active", AgentTaskStatus::Running),
        ("task-command-queued", AgentTaskStatus::Queued),
    ] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Command concurrency fixture {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: None,
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "prompt": "Check local command concurrency",
                        "priority": 3,
                        "order_no": 7,
                        "model": "auto",
                        "agent": "main"
                    }),
                    created_at_ms: TimestampMs(1_783_000_500_000),
                },
            )
            .expect("seed command concurrency task");
    }

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: "task-command-queued".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: Some(1),
        reason: Some("ready from gateway".to_string()),
        payload: Default::default(),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["task"]["task_id"], "task-command-queued");
    assert_eq!(body["task"]["status"], "queued");
    let task_id = TaskId::new("task-command-queued").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load queued task").unwrap();
    assert_eq!(task.status, AgentTaskStatus::Queued);
}

#[tokio::test]
async fn agent_scheduler_gateway_matrix_command_respects_max_concurrent() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-command-matrix-concurrency");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    seed_matrix_worker(&store, &project_path, "matrix-dev");

    for (task_id, status, worker_id) in [
        ("task-command-matrix-active", AgentTaskStatus::Dispatched, Some("matrix-dev")),
        ("task-command-matrix-queued", AgentTaskStatus::Queued, None),
    ] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Matrix command concurrency fixture {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: worker_id
                        .map(|worker_id| WorkerName::new(worker_id).expect("worker id")),
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "prompt": "Check Matrix command concurrency",
                        "priority": 3,
                        "order_no": 7,
                        "model": "auto",
                        "agent": "main"
                    }),
                    created_at_ms: TimestampMs(1_783_000_500_000),
                },
            )
            .expect("seed matrix command concurrency task");
    }
    claim_matrix_run(
        &store,
        &project_path,
        "task-command-matrix-active",
        "run-command-matrix-active",
        "matrix-dev",
    );

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: "task-command-matrix-queued".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Matrix,
        max_concurrent: Some(1),
        reason: Some("ready from gateway".to_string()),
        payload: Default::default(),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["task"]["task_id"], "task-command-matrix-queued");
    assert_eq!(body["task"]["status"], "queued");
    let task_id = TaskId::new("task-command-matrix-queued").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load queued task").unwrap();
    assert_eq!(task.status, AgentTaskStatus::Queued);
}

#[tokio::test]
async fn agent_scheduler_project_wake_runs_registered_queued_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "mock project wake output".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-project-wake").expect("task id"),
                title: "Run queued task through project wake".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-project-wake/spec.md".to_string(),
                meta_uri: "shared/tasks/task-project-wake/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Run this registered queued task through project wake",
                    "priority": 4,
                    "order_no": 9,
                    "model": "auto",
                    "agent": "main"
                }),
                created_at_ms: TimestampMs(1_783_000_900_000),
            },
        )
        .expect("seed queued scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let wake_uri = format!("/agent-scheduler/projects/{project_id}/wake");
    let request = AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(1_783_000_901_000),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        auto_promote_pool_tasks: false,
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(wake_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("wake json")))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["dispatched"], true);
    assert_eq!(body["task_id"], "task-project-wake");
    assert_eq!(body["worker_id"], LOCAL_GATEWAY_WORKER_ID);

    let task_id = TaskId::new("task-project-wake").expect("task id");
    let logs =
        wait_for_store_log(&store, &project_path, &task_id, "local execution submitted result")
            .await;
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert!(logs.iter().any(|log| {
        log.from_status == Some(AgentTaskStatus::Running)
            && log.to_status == Some(AgentTaskStatus::Submitted)
            && log.message == "local execution submitted result"
    }));
    let artifacts =
        wait_for_store_artifact(&store, &project_path, &task_id, "execution_result").await;
    assert!(artifacts.iter().any(|artifact| {
        artifact.kind == "execution_result"
            && artifact.payload["success"] == true
            && artifact.payload["output"] == "mock project wake output"
    }));
    let snapshot = get_json(app, &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert_eq!(snapshot["tasks"][0]["task_id"], "task-project-wake");
    assert_eq!(snapshot["tasks"][0]["status"], "submitted");
    assert_eq!(snapshot["tasks"][0]["status_key"], "code_complete");
    assert_eq!(snapshot["tasks"][0]["merge_source_branch"], "vw/task/task-project-wake");
    assert_eq!(snapshot["tasks"][0]["selected_worktree_branch"], "vw/task/task-project-wake");
    assert_ne!(
        snapshot["tasks"][0]["selected_worktree_path"].as_str(),
        Some(project_path.to_string_lossy().as_ref())
    );
}

#[tokio::test]
async fn agent_scheduler_project_wake_skips_lifecycle_ineligible_queued_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-ineligible");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unexpected local execution".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "review_only_flow".to_string(),
                label: "Review Only Flow".to_string(),
                statuses: vec![TaskPoolWorkflowStatusDto {
                    key: "review".to_string(),
                    label: "Review".to_string(),
                    role: TaskPoolWorkflowStatusRoleDto::Review,
                    enabled: true,
                    terminal: false,
                    next: None,
                    transitions: Vec::new(),
                }],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-project-wake-review").expect("task id"),
                title: "Skip queued task in review lifecycle stage".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-project-wake-review/spec.md".to_string(),
                meta_uri: "shared/tasks/task-project-wake-review/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Do not run this review-stage queued task",
                    "model": "auto",
                    "agent": "main",
                    "task_board.workflow_id": "review_only_flow",
                    "task_board.status_key": "review",
                    "task_board.status_role": "review",
                }),
                created_at_ms: TimestampMs(1_783_000_902_000),
            },
        )
        .expect("seed review scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let wake_uri = format!("/agent-scheduler/projects/{project_id}/wake");
    let request = AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(1_783_000_903_000),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        auto_promote_pool_tasks: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(wake_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("wake json")))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["dispatched"], false);
    assert_eq!(body.get("task_id"), None);
    let task_id = TaskId::new("task-project-wake-review").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert!(logs.is_empty());
}

#[tokio::test]
async fn agent_scheduler_project_wake_applies_autonomous_staff_assignment() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-staff-assign");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let mut staff_config = StaffConfig::default();
    staff_config.workflow_orchestrations.insert(
        "dev_lifecycle".to_string(),
        StaffWorkflowOrchestrationConfig {
            stage_assignments: vec![StaffStageAssignmentConfig {
                stage_id: "pending".to_string(),
                primary_member_id: Some("staff-orchestrator".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            }],
            ..StaffWorkflowOrchestrationConfig::default()
        },
    );
    let gateway = StoreBackedAgentSchedulerGateway::with_executor_and_staff_config(
        RecordingLocalExecutor { output: "unexpected local execution".to_string() },
        staff_config,
    );
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-autonomous-staff-assign").expect("task id"),
                title: "Assign autonomous stage owner through project wake".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-autonomous-staff-assign/spec.md".to_string(),
                meta_uri: "shared/tasks/task-autonomous-staff-assign/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.status_key": "pending",
                    "task_board.status_role": "queued",
                    "priority": 5,
                    "order_no": 1,
                    "prompt": "Assign the autonomous pending stage owner"
                }),
                created_at_ms: TimestampMs(1_783_000_905_000),
            },
        )
        .expect("seed autonomous stage task");
    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let wake_uri = format!("/agent-scheduler/projects/{project_id}/wake");
    let request = AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(1_783_000_906_000),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        auto_promote_pool_tasks: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(wake_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("wake json")))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;

    let task_id = TaskId::new("task-autonomous-staff-assign").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.metadata["task_board.staff.member_id"], "staff-orchestrator");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    assert_eq!(body["dispatched"], false);
    assert_eq!(body["task_id"], serde_json::Value::Null);
    assert!(store.load_latest_run_for_task(&project_path, &task_id).expect("latest run").is_none());
}

#[tokio::test]
async fn agent_scheduler_project_wake_starts_assigned_autonomous_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-staff-start");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let mut staff_config = StaffConfig::default();
    staff_config.workflow_orchestrations.insert(
        "dev_lifecycle".to_string(),
        StaffWorkflowOrchestrationConfig {
            stage_assignments: vec![StaffStageAssignmentConfig {
                stage_id: "planning".to_string(),
                primary_member_id: Some("staff-planner".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            }],
            ..StaffWorkflowOrchestrationConfig::default()
        },
    );
    let gateway = StoreBackedAgentSchedulerGateway::with_executor_and_staff_config(
        RecordingLocalExecutor { output: "mock assigned staff wake output".to_string() },
        staff_config,
    );
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-assigned-autonomous-start").expect("task id"),
                title: "Start assigned autonomous stage through project wake".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-assigned-autonomous-start/spec.md".to_string(),
                meta_uri: "shared/tasks/task-assigned-autonomous-start/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.status_key": "planning",
                    "task_board.status_role": "planning",
                    "task_board.staff.member_id": "staff-planner",
                    "priority": 4,
                    "order_no": 2,
                    "prompt": "Start the assigned autonomous planning stage"
                }),
                created_at_ms: TimestampMs(1_783_000_907_000),
            },
        )
        .expect("seed assigned autonomous stage task");
    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let wake_uri = format!("/agent-scheduler/projects/{project_id}/wake");
    let request = AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(1_783_000_908_000),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        auto_promote_pool_tasks: false,
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(wake_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("wake json")))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;

    assert_eq!(body["dispatched"], true);
    assert_eq!(body["task_id"], "task-assigned-autonomous-start");
    assert_eq!(body["worker_id"], LOCAL_GATEWAY_WORKER_ID);

    let task_id = TaskId::new("task-assigned-autonomous-start").expect("task id");
    let logs =
        wait_for_store_log(&store, &project_path, &task_id, "local execution submitted result")
            .await;
    assert!(logs.iter().any(|log| {
        log.from_status == Some(AgentTaskStatus::Running)
            && log.to_status == Some(AgentTaskStatus::Submitted)
            && log.message == "local execution submitted result"
    }));
    let events = store.list_events_after(&project_path, None, 100).expect("list events");
    assert!(events.iter().any(|event| {
        event.task_id.as_ref() == Some(&task_id)
            && event.message == "agent scheduler project wake staff continuation"
            && event.payload["command"] == "start"
    }));
}

#[tokio::test]
async fn agent_scheduler_worker_run_event_completes_matrix_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-worker-event");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused local executor".to_string(),
    });
    let store = gateway.store.clone();
    let task_id = TaskId::new("task-matrix-worker-event").expect("task id");
    let run_id = RunId::new("run-task-matrix-worker-event-1").expect("run id");
    let worker_id = WorkerName::new("matrix-local").expect("worker id");

    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "matrix_worker_flow".to_string(),
                label: "Matrix Worker Flow".to_string(),
                statuses: vec![
                    workflow_status_with_transitions(
                        "running",
                        TaskPoolWorkflowStatusRoleDto::Executing,
                        vec![workflow_transition("success", "code_complete")],
                    ),
                    workflow_status_with_transitions(
                        "code_complete",
                        TaskPoolWorkflowStatusRoleDto::ReviewReady,
                        vec![workflow_transition("success", "done")],
                    ),
                    workflow_status_with_transitions(
                        "done",
                        TaskPoolWorkflowStatusRoleDto::Done,
                        Vec::new(),
                    ),
                ],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: task_id.clone(),
                title: "Complete through Matrix worker event".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Dispatched,
                spec_uri: "task-board/task-matrix-worker-event/spec.md".to_string(),
                meta_uri: "task-board/task-matrix-worker-event/meta.json".to_string(),
                assigned_to: Some(worker_id.clone()),
                team_id: None,
                room_id: Some("!room:localhost".to_string()),
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Complete from Matrix worker",
                    "model": "auto",
                    "task_board.status_key": "planning",
                    "task_board.status_role": "planning",
                    "task_board.status_key_map": {
                        "running": "running",
                        "submitted": "code_complete",
                        "completed": "done"
                    }
                }),
                created_at_ms: TimestampMs(1_783_001_000_000),
            },
        )
        .expect("seed dispatched task");
    store
        .claim_task_run(
            &project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: worker_id.clone(),
                lease_acquired_at_ms: TimestampMs(1_783_001_000_100),
                lease_expires_at_ms: Some(TimestampMs(1_783_001_600_100)),
            },
        )
        .expect("claim matrix run")
        .expect("run claimed");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let uri = format!(
        "/agent-scheduler/projects/{project_id}/tasks/{}/runs/{}/worker-events",
        encode_path_segment(task_id.as_str()),
        encode_path_segment(run_id.as_str())
    );

    let started = post_json(
        app.clone(),
        &uri,
        &AgentSchedulerWorkerRunEventRequest {
            platform_id: vw_api_types::id::DEFAULT_PLATFORM_ID.to_string(),
            company_id: vw_api_types::id::DEFAULT_COMPANY_ID.to_string(),
            project_id: project_id.clone(),
            worker_id: worker_id.as_str().to_string(),
            event: AgentSchedulerWorkerRunEventKindDto::Started,
            message: Some("Matrix worker started".to_string()),
            result: None,
            error_message: None,
            workspace: None,
            payload: json!({"room_id": "!room:localhost"}),
            now_ms: Some(TimestampMs(1_783_001_000_200)),
            lease_expires_at_ms: Some(TimestampMs(1_783_001_600_200)),
        },
    )
    .await;
    assert_eq!(started["accepted"], true);
    assert_eq!(started["task"]["status"], "running");
    assert_eq!(started["event"]["kind"], "run_started");

    let completed = post_json(
        app,
        &uri,
        &AgentSchedulerWorkerRunEventRequest {
            platform_id: vw_api_types::id::DEFAULT_PLATFORM_ID.to_string(),
            company_id: vw_api_types::id::DEFAULT_COMPANY_ID.to_string(),
            project_id: project_id.clone(),
            worker_id: worker_id.as_str().to_string(),
            event: AgentSchedulerWorkerRunEventKindDto::Completed,
            message: Some("Matrix worker completed".to_string()),
            result: Some("matrix worker output".to_string()),
            error_message: None,
            workspace: None,
            payload: json!({"room_id": "!room:localhost"}),
            now_ms: Some(TimestampMs(1_783_001_000_300)),
            lease_expires_at_ms: None,
        },
    )
    .await;
    assert_eq!(completed["accepted"], true);
    assert_eq!(completed["task"]["status"], "completed");
    assert_eq!(completed["task"]["status_key"], "done");
    assert_eq!(completed["run"]["status"], "succeeded");
    assert_eq!(completed["event"]["kind"], "run_completed");

    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    let run = store.load_run(&project_path, &run_id).expect("load run").expect("run");
    assert_eq!(run.status, AgentRunStatus::Succeeded);
    assert_eq!(run.finished_at_ms, Some(TimestampMs(1_783_001_000_300)));
    let artifacts = store.list_task_artifacts(&project_path, &task_id).expect("list artifacts");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].kind, "execution_result");
    assert_eq!(artifacts[0].payload["source"], "matrix_worker");
    assert_eq!(artifacts[0].payload["output"], "matrix worker output");
    let logs = store.list_task_logs(&project_path, &task_id).expect("list logs");
    assert!(logs.iter().any(|log| log.to_status == Some(AgentTaskStatus::Acknowledged)));
    assert!(logs.iter().any(|log| log.to_status == Some(AgentTaskStatus::Running)));
    assert!(logs.iter().any(|log| log.to_status == Some(AgentTaskStatus::Completed)));
}

#[tokio::test]
async fn worker_submitted_advances_by_lifecycle_success_transition() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-worker-outcome-success");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    let task_id = TaskId::new("task-worker-outcome-success").expect("task id");
    let run_id = RunId::new("run-worker-outcome-success").expect("run id");
    let worker_id = WorkerName::new("matrix-outcome").expect("worker id");

    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "worker_outcome_flow".to_string(),
                label: "Worker Outcome Flow".to_string(),
                statuses: vec![
                    workflow_status_with_transitions(
                        "custom_running",
                        TaskPoolWorkflowStatusRoleDto::Executing,
                        vec![workflow_transition("success", "custom_review_ready")],
                    ),
                    workflow_status_with_transitions(
                        "custom_review_ready",
                        TaskPoolWorkflowStatusRoleDto::ReviewReady,
                        Vec::new(),
                    ),
                ],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: task_id.clone(),
                title: "Submit through lifecycle outcome".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Running,
                spec_uri: "shared/tasks/task-worker-outcome-success/spec.md".to_string(),
                meta_uri: "shared/tasks/task-worker-outcome-success/meta.json".to_string(),
                assigned_to: Some(worker_id.clone()),
                team_id: None,
                room_id: Some("!room:localhost".to_string()),
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.workflow_id": "worker_outcome_flow",
                    "task_board.status_key": "custom_running",
                    "task_board.status_role": "executing",
                    "prompt": "Submit through lifecycle outcome"
                }),
                created_at_ms: TimestampMs(1_783_001_100_000),
            },
        )
        .expect("seed running task");
    store
        .claim_task_run(
            &project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: worker_id.clone(),
                lease_acquired_at_ms: TimestampMs(1_783_001_100_100),
                lease_expires_at_ms: Some(TimestampMs(1_783_001_700_100)),
            },
        )
        .expect("claim run")
        .expect("run claimed");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let uri = format!(
        "/agent-scheduler/projects/{project_id}/tasks/{}/runs/{}/worker-events",
        encode_path_segment(task_id.as_str()),
        encode_path_segment(run_id.as_str())
    );

    let submitted = post_json(
        app,
        &uri,
        &AgentSchedulerWorkerRunEventRequest {
            platform_id: vw_api_types::id::DEFAULT_PLATFORM_ID.to_string(),
            company_id: vw_api_types::id::DEFAULT_COMPANY_ID.to_string(),
            project_id: project_id.clone(),
            worker_id: worker_id.as_str().to_string(),
            event: AgentSchedulerWorkerRunEventKindDto::Submitted,
            message: Some("Matrix worker submitted by outcome".to_string()),
            result: Some("custom workflow result".to_string()),
            error_message: None,
            workspace: None,
            payload: json!({"room_id": "!room:localhost"}),
            now_ms: Some(TimestampMs(1_783_001_100_200)),
            lease_expires_at_ms: None,
        },
    )
    .await;

    assert_eq!(submitted["accepted"], true);
    assert_eq!(submitted["task"]["status"], "submitted");
    assert_eq!(submitted["task"]["status_key"], "custom_review_ready");
    assert_eq!(submitted["task"]["read_model"]["status_role"], "review_ready");
    assert_eq!(submitted["event"]["from_status_key"], "custom_running");
    assert_eq!(submitted["event"]["to_status_key"], "custom_review_ready");

    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "custom_review_ready");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");
    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert!(logs.iter().any(|log| {
        log.from_status_key.as_deref() == Some("custom_running")
            && log.to_status_key.as_deref() == Some("custom_review_ready")
    }));
}

#[tokio::test]
async fn worker_failed_advances_by_lifecycle_failure_transition() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-worker-outcome-failure");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    let task_id = TaskId::new("task-worker-outcome-failure").expect("task id");
    let run_id = RunId::new("run-worker-outcome-failure").expect("run id");
    let worker_id = WorkerName::new("matrix-outcome").expect("worker id");

    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "worker_outcome_flow".to_string(),
                label: "Worker Outcome Flow".to_string(),
                statuses: vec![
                    workflow_status_with_transitions(
                        "custom_running",
                        TaskPoolWorkflowStatusRoleDto::Executing,
                        vec![workflow_transition("failure", "custom_failed")],
                    ),
                    workflow_status_with_transitions(
                        "custom_failed",
                        TaskPoolWorkflowStatusRoleDto::Failed,
                        Vec::new(),
                    ),
                ],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: task_id.clone(),
                title: "Fail through lifecycle outcome".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Running,
                spec_uri: "shared/tasks/task-worker-outcome-failure/spec.md".to_string(),
                meta_uri: "shared/tasks/task-worker-outcome-failure/meta.json".to_string(),
                assigned_to: Some(worker_id.clone()),
                team_id: None,
                room_id: Some("!room:localhost".to_string()),
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.workflow_id": "worker_outcome_flow",
                    "task_board.status_key": "custom_running",
                    "task_board.status_role": "executing",
                    "prompt": "Fail through lifecycle outcome"
                }),
                created_at_ms: TimestampMs(1_783_001_101_000),
            },
        )
        .expect("seed running task");
    store
        .claim_task_run(
            &project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: worker_id.clone(),
                lease_acquired_at_ms: TimestampMs(1_783_001_101_100),
                lease_expires_at_ms: Some(TimestampMs(1_783_001_701_100)),
            },
        )
        .expect("claim run")
        .expect("run claimed");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let uri = format!(
        "/agent-scheduler/projects/{project_id}/tasks/{}/runs/{}/worker-events",
        encode_path_segment(task_id.as_str()),
        encode_path_segment(run_id.as_str())
    );

    let failed = post_json(
        app,
        &uri,
        &AgentSchedulerWorkerRunEventRequest {
            platform_id: vw_api_types::id::DEFAULT_PLATFORM_ID.to_string(),
            company_id: vw_api_types::id::DEFAULT_COMPANY_ID.to_string(),
            project_id: project_id.clone(),
            worker_id: worker_id.as_str().to_string(),
            event: AgentSchedulerWorkerRunEventKindDto::Failed,
            message: Some("Matrix worker failed by outcome".to_string()),
            result: None,
            error_message: Some("custom failure".to_string()),
            workspace: None,
            payload: json!({"room_id": "!room:localhost"}),
            now_ms: Some(TimestampMs(1_783_001_101_200)),
            lease_expires_at_ms: None,
        },
    )
    .await;

    assert_eq!(failed["accepted"], true);
    assert_eq!(failed["task"]["status"], "failed");
    assert_eq!(failed["task"]["status_key"], "custom_failed");
    assert_eq!(failed["event"]["from_status_key"], "custom_running");
    assert_eq!(failed["event"]["to_status_key"], "custom_failed");

    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Failed);
    assert_eq!(task.metadata["task_board.status_key"], "custom_failed");
    assert_eq!(task.metadata["task_board.status_role"], "failed");
}

#[tokio::test]
async fn agent_scheduler_project_wake_updates_task_board_projection_from_initial_pending_map() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-projection");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "mock task board projection output".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-project-wake-projection").expect("task id"),
                title: "Run queued task with task board projection".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-project-wake-projection/spec.md".to_string(),
                meta_uri: "shared/tasks/task-project-wake-projection/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Run this registered queued task through project wake",
                    "priority": 4,
                    "order_no": 9,
                    "model": "auto",
                    "agent": "main",
                    "task_board.status_key": "pending",
                    "task_board.status_role": "queued",
                    "task_board.status_key_map": {
                        "running": "running",
                        "submitted": "code_complete"
                    }
                }),
                created_at_ms: TimestampMs(1_783_000_900_000),
            },
        )
        .expect("seed queued scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let wake_uri = format!("/agent-scheduler/projects/{project_id}/wake");
    let request = AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(1_783_000_901_000),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        auto_promote_pool_tasks: false,
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(wake_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("wake json")))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["dispatched"], true);
    assert_eq!(body["task_id"], "task-project-wake-projection");

    let task_id = TaskId::new("task-project-wake-projection").expect("task id");
    wait_for_store_log(&store, &project_path, &task_id, "local execution submitted result").await;
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");

    let snapshot = get_json(app, &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert_eq!(snapshot["tasks"][0]["task_id"], "task-project-wake-projection");
    assert_eq!(snapshot["tasks"][0]["status"], "submitted");
    assert_eq!(snapshot["tasks"][0]["status_key"], "code_complete");
    assert_eq!(snapshot["tasks"][0]["read_model"]["status_role"], "review_ready");
}

#[tokio::test]
async fn agent_scheduler_project_wake_respects_local_max_concurrent() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-concurrency");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "should not start behind concurrency limit".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");

    for (task_id, status) in [
        ("task-active-local", AgentTaskStatus::Running),
        ("task-queued-behind-limit", AgentTaskStatus::Queued),
    ] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Concurrency fixture {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: None,
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "prompt": "Check local gateway concurrency",
                        "priority": 4,
                        "order_no": 9,
                        "model": "auto",
                        "agent": "main"
                    }),
                    created_at_ms: TimestampMs(1_783_000_910_000),
                },
            )
            .expect("seed concurrency task");
    }
    store
        .claim_task_run(
            &project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: RunId::new("run-active-local").expect("run id"),
                task_id: TaskId::new("task-active-local").expect("task id"),
                worker_id: WorkerName::new(LOCAL_GATEWAY_WORKER_ID).expect("worker id"),
                lease_acquired_at_ms: TimestampMs(1_783_000_910_500),
                lease_expires_at_ms: Some(TimestampMs(1_783_000_940_500)),
            },
        )
        .expect("claim active local run")
        .expect("active local run claimed");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/wake"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_911_000),
                        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                        max_concurrent: Some(1),
                        auto_promote_pool_tasks: false,
                    })
                    .expect("wake json"),
                ))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["dispatched"], false);
    let queued_id = TaskId::new("task-queued-behind-limit").expect("task id");
    let queued = store.load_task(&project_path, &queued_id).expect("load queued task").unwrap();
    assert_eq!(queued.status, AgentTaskStatus::Queued);
}

#[tokio::test]
async fn agent_scheduler_project_wake_ignores_matrix_runs_for_local_concurrency() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-local-matrix-concurrency");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "local task should run despite matrix lease".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");

    for (task_id, status, worker_id) in [
        ("task-active-matrix", AgentTaskStatus::Dispatched, Some("matrix-local")),
        ("task-local-queued", AgentTaskStatus::Queued, None),
    ] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Local concurrency fixture {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: worker_id
                        .map(|worker_id| WorkerName::new(worker_id).expect("worker id")),
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "prompt": "Check local wake ignores Matrix lease",
                        "priority": 4,
                        "order_no": 9,
                        "model": "auto",
                        "agent": "main"
                    }),
                    created_at_ms: TimestampMs(1_783_000_910_000),
                },
            )
            .expect("seed concurrency task");
    }
    store
        .claim_task_run(
            &project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: RunId::new("run-active-matrix").expect("run id"),
                task_id: TaskId::new("task-active-matrix").expect("task id"),
                worker_id: WorkerName::new("matrix-local").expect("worker id"),
                lease_acquired_at_ms: TimestampMs(1_783_000_910_500),
                lease_expires_at_ms: Some(TimestampMs(1_783_000_940_500)),
            },
        )
        .expect("claim active matrix run")
        .expect("active matrix run claimed");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/wake"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_911_000),
                        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                        max_concurrent: Some(1),
                        auto_promote_pool_tasks: false,
                    })
                    .expect("wake json"),
                ))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["dispatched"], true);
    assert_eq!(body["task_id"], "task-local-queued");
    let task_id = TaskId::new("task-local-queued").expect("task id");
    wait_for_store_log(&store, &project_path, &task_id, "local execution submitted result").await;
    let task = store.load_task(&project_path, &task_id).expect("load queued task").unwrap();
    assert_eq!(task.status, AgentTaskStatus::Submitted);
}

#[tokio::test]
async fn agent_scheduler_project_wake_respects_matrix_max_concurrent() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-matrix-concurrency");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    seed_matrix_worker(&store, &project_path, "matrix-dev");

    for (task_id, status, worker_id) in [
        ("task-active-matrix-wake", AgentTaskStatus::Dispatched, Some("matrix-dev")),
        ("task-matrix-queued-behind-limit", AgentTaskStatus::Queued, None),
    ] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Matrix wake concurrency fixture {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: worker_id
                        .map(|worker_id| WorkerName::new(worker_id).expect("worker id")),
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "prompt": "Check Matrix wake concurrency",
                        "priority": 4,
                        "order_no": 9,
                        "model": "auto",
                        "agent": "main"
                    }),
                    created_at_ms: TimestampMs(1_783_000_910_000),
                },
            )
            .expect("seed matrix wake concurrency task");
    }
    claim_matrix_run(
        &store,
        &project_path,
        "task-active-matrix-wake",
        "run-active-matrix-wake",
        "matrix-dev",
    );

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/wake"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_911_000),
                        dispatch_mode: AgentSchedulerDispatchModeDto::Matrix,
                        max_concurrent: Some(1),
                        auto_promote_pool_tasks: false,
                    })
                    .expect("wake json"),
                ))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["dispatched"], false);
    let queued_id = TaskId::new("task-matrix-queued-behind-limit").expect("task id");
    let queued = store.load_task(&project_path, &queued_id).expect("load queued task").unwrap();
    assert_eq!(queued.status, AgentTaskStatus::Queued);
}

#[tokio::test]
async fn agent_scheduler_gateway_policy_wake_leaves_pool_when_auto_promote_disabled() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-policy-pool-disabled");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "should not execute disabled pool task".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-policy-pool-disabled-wake").expect("task id"),
                title: "Policy leaves pool task disabled".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Draft,
                spec_uri: "shared/tasks/task-policy-pool-disabled-wake/spec.md".to_string(),
                meta_uri: "shared/tasks/task-policy-pool-disabled-wake/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Stay in pool unless auto promote is enabled",
                    "task_board.status_key": "pool",
                    "task_board.status_role": "backlog",
                    "auto_promote_delay_ms": 0
                }),
                created_at_ms: TimestampMs(1_783_000_100_000),
            },
        )
        .expect("seed pool scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/wake"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_140_000),
                        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                        max_concurrent: None,
                        auto_promote_pool_tasks: false,
                    })
                    .expect("wake json"),
                ))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["dispatched"], false);

    let task_id = TaskId::new("task-policy-pool-disabled-wake").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Draft);
    let events = store.list_events_after(&project_path, None, 100).expect("events");
    assert!(events.iter().all(|event| event.payload["policy_action"] != "auto_promote_pool"));
}

#[tokio::test]
async fn agent_scheduler_gateway_policy_wake_leaves_pool_task_draft() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-policy-pool");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "policy promoted execution output".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-policy-pool-wake").expect("task id"),
                title: "Policy promotes pool task".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Draft,
                spec_uri: "shared/tasks/task-policy-pool-wake/spec.md".to_string(),
                meta_uri: "shared/tasks/task-policy-pool-wake/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Run after policy promotes from pool",
                    "task_board.status_key": "pool",
                    "task_board.status_role": "backlog",
                    "auto_promote_delay_ms": 0
                }),
                created_at_ms: TimestampMs(1_783_000_100_000),
            },
        )
        .expect("seed pool scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/wake"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_140_000),
                        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                        max_concurrent: None,
                        auto_promote_pool_tasks: true,
                    })
                    .expect("wake json"),
                ))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["dispatched"], false);
    assert_eq!(body["task_id"], serde_json::Value::Null);

    let task_id = TaskId::new("task-policy-pool-wake").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Draft);
    let events = store.list_events_after(&project_path, None, 100).expect("events");
    assert!(events.iter().all(|event| event.payload["policy_action"] != "auto_promote_pool"));
}

#[tokio::test]
async fn agent_scheduler_gateway_policy_leaves_review_ready_task_submitted() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-policy-merge");
    init_merge_test_repo(&project_path, "feature/task-policy-merge");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    let mut task =
        submitted_merge_task("task-policy-merge", &project_path, "feature/task-policy-merge", true);
    task.metadata.as_object_mut().expect("metadata object").extend([
        ("task_board.status_key".to_string(), json!("code_complete")),
        ("task_board.status_role".to_string(), json!("review_ready")),
    ]);
    store.create_task(&project_path, task).expect("seed submitted scheduler task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/wake"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_850_000),
                        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                        max_concurrent: None,
                        auto_promote_pool_tasks: false,
                    })
                    .expect("wake json"),
                ))
                .expect("wake request"),
        )
        .await
        .expect("wake response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["dispatched"], false);

    let task_id = TaskId::new("task-policy-merge").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert!(!project_path.join("feature.txt").exists());
    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert!(logs.is_empty());
    let events = store.list_events_after(&project_path, None, 100).expect("events");
    assert!(events.iter().all(|event| event.payload["policy_action"] != "skip_review"));
    let holder = super::super::task_pool::task_worktree_pool_service()
        .merge_lock_holder(&project_info, "main")
        .await
        .expect("merge lock holder");
    assert_eq!(holder, None);
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_task_board_uses_task_board_status_key_metadata() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-workflow-status");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");

    for (task_id, status) in [
        ("task-campaign-running", AgentTaskStatus::Running),
        ("task-campaign-review", AgentTaskStatus::Submitted),
    ] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Workflow mapped task {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("task-board/{task_id}/spec.md"),
                    meta_uri: format!("task-board/{task_id}/meta.json"),
                    assigned_to: None,
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "prompt": "Run this workflow task",
                        "priority": 2,
                        "order_no": 5,
                        "model": "auto",
                        "task_board.workflow_id": "marketing_campaign",
                        "task_board.status_key": "campaign_queued",
                        "task_board.status_role": "queued",
                        "task_board.status_key_map": {
                            "queued": "campaign_queued",
                            "running": "campaign_running",
                            "submitted": "campaign_review_ready"
                        }
                    }),
                    created_at_ms: TimestampMs(1_783_000_910_000),
                },
            )
            .expect("seed scheduler task");
    }

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let snapshot = get_json(app, &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    let tasks = snapshot["tasks"].as_array().expect("tasks array");
    let status_key = |task_id: &str| {
        tasks
            .iter()
            .find(|task| task["task_id"] == task_id)
            .and_then(|task| task["status_key"].as_str())
            .expect("task status key")
    };

    assert_eq!(status_key("task-campaign-running"), "campaign_queued");
    assert_eq!(status_key("task-campaign-review"), "campaign_queued");
}

#[tokio::test]
async fn agent_scheduler_run_stage_action_applies_success_outcome_binding() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-stage-action-success");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "stage_action_flow".to_string(),
                label: "Stage Action Flow".to_string(),
                statuses: vec![
                    TaskPoolWorkflowStatusDto {
                        key: "queued".to_string(),
                        label: "Queued".to_string(),
                        role: TaskPoolWorkflowStatusRoleDto::Queued,
                        enabled: true,
                        terminal: false,
                        next: Some("code_complete".to_string()),
                        transitions: vec![TaskPoolWorkflowTransitionDto {
                            outcome: "success".to_string(),
                            label: None,
                            target: "code_complete".to_string(),
                        }],
                    },
                    TaskPoolWorkflowStatusDto {
                        key: "code_complete".to_string(),
                        label: "Code Complete".to_string(),
                        role: TaskPoolWorkflowStatusRoleDto::ReviewReady,
                        enabled: true,
                        terminal: false,
                        next: None,
                        transitions: Vec::new(),
                    },
                ],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-stage-action-success").expect("task id"),
                title: "Apply stage action outcome".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-stage-action-success/spec.md".to_string(),
                meta_uri: "shared/tasks/task-stage-action-success/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.workflow_id": "stage_action_flow",
                    "task_board.status_key": "queued",
                    "task_board.status_role": "queued",
                    "priority": 3,
                    "order_no": 1,
                    "prompt": "Advance after stage action success"
                }),
                created_at_ms: TimestampMs(1_783_000_915_000),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command = AgentSchedulerCommandRequest {
        task_id: "task-stage-action-success".into(),
        command: AgentSchedulerCommandKindDto::RunStageAction,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("stage action reported success".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
                stage_id: "queued".to_string(),
                capability_id: "execute_task".to_string(),
                workflow_uuid: None,
                inputs: AgentSchedulerStageActionInputsDto {
                    workflow_uuid: None,
                    parameters: BTreeMap::from([("api_token".to_string(), json!("secret-token"))]),
                },
                outcome: Some(AgentSchedulerStageActionOutcomeDto::Success),
                member_id: Some("staff-reviewer".to_string()),
                metadata: Some(json!({"source": "stage_action_result"})),
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/commands"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["task"]["status"], "submitted");
    assert_eq!(body["task"]["status_key"], "code_complete");
    assert_eq!(body["event"]["from_status_key"], "queued");
    assert_eq!(body["event"]["to_status_key"], "code_complete");
    assert_eq!(body["event"]["payload"]["stage_action"]["outcome"], "success");
    assert!(!body["event"]["payload"].to_string().contains("secret-token"));

    let task_id = TaskId::new("task-stage-action-success").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "code_complete");
    assert_eq!(task.metadata["task_board.stage_action.capability_id"], "execute_task");
    assert_eq!(task.metadata["task_board.stage_action.stage_id"], "queued");
    assert_eq!(task.metadata["task_board.stage_action.outcome"], "success");
    assert!(!task.metadata.to_string().contains("secret-token"));
}

#[tokio::test]
async fn agent_scheduler_run_stage_action_uses_lifecycle_role_runtime_mapping() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-stage-action-custom-role");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "custom_stage_action_flow".to_string(),
                label: "Custom Stage Action Flow".to_string(),
                statuses: vec![
                    TaskPoolWorkflowStatusDto {
                        key: "custom_queued".to_string(),
                        label: "Custom Queued".to_string(),
                        role: TaskPoolWorkflowStatusRoleDto::Queued,
                        enabled: true,
                        terminal: false,
                        next: Some("custom_review_ready".to_string()),
                        transitions: vec![TaskPoolWorkflowTransitionDto {
                            outcome: "success".to_string(),
                            label: None,
                            target: "custom_review_ready".to_string(),
                        }],
                    },
                    TaskPoolWorkflowStatusDto {
                        key: "custom_review_ready".to_string(),
                        label: "Custom Review Ready".to_string(),
                        role: TaskPoolWorkflowStatusRoleDto::ReviewReady,
                        enabled: true,
                        terminal: false,
                        next: None,
                        transitions: Vec::new(),
                    },
                ],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-stage-action-custom-role").expect("task id"),
                title: "Apply custom stage action outcome".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-stage-action-custom-role/spec.md".to_string(),
                meta_uri: "shared/tasks/task-stage-action-custom-role/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.workflow_id": "custom_stage_action_flow",
                    "task_board.status_key": "custom_queued",
                    "task_board.status_role": "queued",
                    "priority": 3,
                    "order_no": 1,
                    "prompt": "Advance custom stage action by lifecycle role"
                }),
                created_at_ms: TimestampMs(1_783_000_915_500),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command = AgentSchedulerCommandRequest {
        task_id: "task-stage-action-custom-role".into(),
        command: AgentSchedulerCommandKindDto::RunStageAction,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("stage action reported success".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
                stage_id: "custom_queued".to_string(),
                capability_id: "execute_task".to_string(),
                workflow_uuid: None,
                inputs: AgentSchedulerStageActionInputsDto::default(),
                outcome: Some(AgentSchedulerStageActionOutcomeDto::Success),
                member_id: Some("staff-reviewer".to_string()),
                metadata: None,
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/commands"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["task"]["status"], "submitted");
    assert_eq!(body["task"]["status_key"], "custom_review_ready");
    assert_eq!(body["task"]["read_model"]["status_role"], "review_ready");
    assert_eq!(body["event"]["from_status_key"], "custom_queued");
    assert_eq!(body["event"]["to_status_key"], "custom_review_ready");

    let task_id = TaskId::new("task-stage-action-custom-role").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "custom_review_ready");
    assert_eq!(task.metadata["task_board.status_role"], "review_ready");
}

#[tokio::test]
async fn agent_scheduler_run_stage_action_uses_builtin_status_role_when_metadata_is_missing() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-stage-action-builtin-role");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-stage-action-builtin-role").expect("task id"),
                title: "Run builtin planning stage action".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-stage-action-builtin-role/spec.md".to_string(),
                meta_uri: "shared/tasks/task-stage-action-builtin-role/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.status_key": "planning",
                    "priority": 2,
                    "order_no": 1,
                    "prompt": "Run builtin planning action without role metadata"
                }),
                created_at_ms: TimestampMs(1_783_000_915_650),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command = AgentSchedulerCommandRequest {
        task_id: "task-stage-action-builtin-role".into(),
        command: AgentSchedulerCommandKindDto::RunStageAction,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("accepted builtin planning stage action".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
                stage_id: "planning".to_string(),
                capability_id: "execute_task".to_string(),
                workflow_uuid: None,
                inputs: AgentSchedulerStageActionInputsDto::default(),
                outcome: None,
                member_id: Some("staff-planner".to_string()),
                metadata: None,
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/commands"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["task"]["status_key"], "planning");
    assert_eq!(body["event"]["payload"]["stage_action"]["capability_id"], "execute_task");
    assert_eq!(body["event"]["payload"]["stage_action"]["result"], "accepted");
}

#[tokio::test]
async fn agent_scheduler_run_stage_action_accepts_project_custom_capability() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-stage-action-custom-capability");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let mut staff_config = StaffConfig::default();
    staff_config.workflow_orchestrations.insert(
        "release_flow".to_string(),
        StaffWorkflowOrchestrationConfig {
            capabilities: vec![StaffCapabilityConfig {
                id: "release_workflow".to_string(),
                label: "Release Workflow".to_string(),
                capability_type: StaffCapabilityKindConfig::SavedWorkflow,
                built_in: false,
                input_summary: StaffCapabilityIoSummaryConfig::default(),
                result_summary: StaffCapabilityIoSummaryConfig::default(),
            }],
            ..StaffWorkflowOrchestrationConfig::default()
        },
    );
    let gateway = StoreBackedAgentSchedulerGateway::with_executor_and_staff_config(
        RecordingLocalExecutor { output: "stage action should not run local executor".to_string() },
        staff_config,
    );
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "release_flow".to_string(),
                label: "Release Flow".to_string(),
                statuses: vec![TaskPoolWorkflowStatusDto {
                    key: "release_planning".to_string(),
                    label: "Release Planning".to_string(),
                    role: TaskPoolWorkflowStatusRoleDto::Planning,
                    enabled: true,
                    terminal: false,
                    next: None,
                    transitions: Vec::new(),
                }],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-stage-action-custom-capability").expect("task id"),
                title: "Run custom saved workflow capability".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-stage-action-custom-capability/spec.md".to_string(),
                meta_uri: "shared/tasks/task-stage-action-custom-capability/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.workflow_id": "release_flow",
                    "task_board.status_key": "release_planning",
                    "task_board.status_role": "planning",
                    "priority": 2,
                    "order_no": 1,
                    "prompt": "Run configured release workflow"
                }),
                created_at_ms: TimestampMs(1_783_000_915_800),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command = AgentSchedulerCommandRequest {
        task_id: "task-stage-action-custom-capability".into(),
        command: AgentSchedulerCommandKindDto::RunStageAction,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("accepted custom stage workflow".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
                stage_id: "release_planning".to_string(),
                capability_id: "release_workflow".to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099".to_string()),
                inputs: AgentSchedulerStageActionInputsDto {
                    workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099".to_string()),
                    parameters: BTreeMap::from([("release_channel".to_string(), json!("stable"))]),
                },
                outcome: None,
                member_id: Some("staff-planner".to_string()),
                metadata: None,
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/commands"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["task"]["status_key"], "release_planning");
    assert_eq!(body["event"]["payload"]["stage_action"]["capability_id"], "release_workflow");
    assert_eq!(body["event"]["payload"]["stage_action"]["result"], "accepted");

    let task_id = TaskId::new("task-stage-action-custom-capability").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    assert_eq!(task.metadata["task_board.stage_action.capability_id"], "release_workflow");
    assert_eq!(
        task.metadata["task_board.stage_action.workflow_uuid"],
        "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099"
    );
}

#[tokio::test]
async fn agent_scheduler_run_stage_action_rejects_missing_outcome_transition() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-stage-action-reject");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "stage_action_flow".to_string(),
                label: "Stage Action Flow".to_string(),
                statuses: vec![TaskPoolWorkflowStatusDto {
                    key: "queued".to_string(),
                    label: "Queued".to_string(),
                    role: TaskPoolWorkflowStatusRoleDto::Queued,
                    enabled: true,
                    terminal: false,
                    next: None,
                    transitions: Vec::new(),
                }],
            },
        )
        .expect("save workflow");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-stage-action-reject").expect("task id"),
                title: "Reject unknown stage action outcome".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-stage-action-reject/spec.md".to_string(),
                meta_uri: "shared/tasks/task-stage-action-reject/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.workflow_id": "stage_action_flow",
                    "task_board.status_key": "queued",
                    "priority": 1,
                    "order_no": 1
                }),
                created_at_ms: TimestampMs(1_783_000_916_000),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command = AgentSchedulerCommandRequest {
        task_id: "task-stage-action-reject".into(),
        command: AgentSchedulerCommandKindDto::RunStageAction,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("stage action reported success".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
                stage_id: "queued".to_string(),
                capability_id: "execute_task".to_string(),
                workflow_uuid: None,
                inputs: AgentSchedulerStageActionInputsDto::default(),
                outcome: Some(AgentSchedulerStageActionOutcomeDto::Success),
                member_id: Some("staff-reviewer".to_string()),
                metadata: None,
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/commands"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert!(
        body["error"]
            .as_str()
            .expect("error")
            .contains("stage action outcome success has no workflow transition from queued")
    );

    let task_id = TaskId::new("task-stage-action-reject").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    assert_eq!(task.metadata["task_board.status_key"], "queued");
    assert!(task.metadata.get("task_board.stage_action.outcome").is_none());
}

#[tokio::test]
async fn agent_scheduler_gateway_task_board_updates_moves_and_soft_deletes_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-board-writes");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-board-write").expect("task id"),
                title: "Original board task".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-board-write/spec.md".to_string(),
                meta_uri: "shared/tasks/task-board-write/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.status_key": "backlog",
                    "priority": 9,
                    "order_no": 4,
                    "prompt": "Original prompt"
                }),
                created_at_ms: TimestampMs(1_783_000_910_000),
            },
        )
        .expect("seed scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let metadata_uri =
        format!("/agent-scheduler/projects/{project_id}/tasks/task-board-write/metadata");
    let patch = AgentSchedulerTaskMetadataUpdateRequest {
        patch: AgentSchedulerTaskMetadataPatchDto {
            title: Some("Edited board task".to_string()),
            status_key: Some("review".to_string()),
            priority: Some(3),
            order_no: Some(2),
            workflow_id: Some("dev_lifecycle".to_string()),
            status_role: Some(TaskPoolWorkflowStatusRoleDto::Review),
            metadata_merge: Some(json!({"custom_field": "kept"})),
            ..Default::default()
        },
        reason: Some("metadata edit".to_string()),
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(&metadata_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&patch).expect("patch json")))
                .expect("metadata request"),
        )
        .await
        .expect("metadata response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["task"]["status_key"], "review");
    assert_eq!(body["task"]["priority"], 3);
    assert_eq!(body["task"]["read_model"]["workflow_id"], "dev_lifecycle");
    assert_eq!(body["task"]["read_model"]["status_role"], "review");
    assert_eq!(body["event"]["to_status_key"], "review");

    let move_command = AgentSchedulerCommandRequest {
        task_id: "task-board-write".into(),
        command: AgentSchedulerCommandKindDto::Move,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("manual move".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            move_task: Some(AgentSchedulerMoveTaskPayloadDto {
                from_status_key: Some("review".to_string()),
                to_status_key: "code_complete".to_string(),
                to_status: Some(AgentTaskStatusDto::Submitted),
                order_no: Some(1),
            }),
            ..Default::default()
        },
    };
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&move_command).expect("move json")))
                .expect("move request"),
        )
        .await
        .expect("move response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["task"]["status"], "submitted");
    assert_eq!(body["task"]["status_key"], "code_complete");
    assert_eq!(body["event"]["from_status_key"], "review");
    assert_eq!(body["event"]["to_status_key"], "code_complete");

    let delete_command = AgentSchedulerCommandRequest {
        task_id: "task-board-write".into(),
        command: AgentSchedulerCommandKindDto::SoftDelete,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("delete from board".to_string()),
        payload: Default::default(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&delete_command).expect("delete json")))
                .expect("delete request"),
        )
        .await
        .expect("delete response");
    assert_eq!(response.status(), StatusCode::OK);
    let snapshot =
        get_json(app.clone(), &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert!(snapshot["tasks"].as_array().expect("tasks").is_empty());
    let logs = get_json(
        app,
        &format!("/agent-scheduler/projects/{project_id}/tasks/task-board-write/logs"),
    )
    .await;
    assert!(logs["logs"].as_array().expect("logs").iter().any(|log| {
        log["from_status_key"] == "review" || log["to_status_key"] == "code_complete"
    }));
}

#[tokio::test]
async fn agent_scheduler_gateway_task_board_bulk_archives_and_deletes_tasks() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-bulk-writes");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    for task_id in ["task-bulk-archive", "task-bulk-delete"] {
        store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Bulk task {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status: AgentTaskStatus::Queued,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: None,
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({
                        "task_board.status_key": "backlog",
                        "priority": 5,
                        "order_no": 1
                    }),
                    created_at_ms: TimestampMs(1_783_000_920_000),
                },
            )
            .expect("seed bulk task");
    }

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let bulk = AgentSchedulerBulkCommandRequest {
        stop_on_error: false,
        commands: vec![
            AgentSchedulerCommandRequest {
                task_id: "task-bulk-archive".into(),
                command: AgentSchedulerCommandKindDto::Archive,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason: Some("bulk archive".to_string()),
                payload: Default::default(),
            },
            AgentSchedulerCommandRequest {
                task_id: "task-bulk-delete".into(),
                command: AgentSchedulerCommandKindDto::SoftDelete,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason: Some("bulk delete".to_string()),
                payload: Default::default(),
            },
        ],
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/agent-scheduler/projects/{project_id}/commands/bulk"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&bulk).expect("bulk json")))
                .expect("bulk request"),
        )
        .await
        .expect("bulk response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["accepted"], 2);
    assert_eq!(body["rejected"], 0);

    let snapshot = get_json(app, &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert!(snapshot["tasks"].as_array().expect("tasks").is_empty());
}

#[tokio::test]
async fn agent_scheduler_gateway_legacy_import_is_idempotent_and_persists_workflow() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-legacy-import");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "content_flow".to_string(),
        label: "Content Flow".to_string(),
        statuses: vec![TaskPoolWorkflowStatusDto {
            key: "content_writing".to_string(),
            label: "Writing".to_string(),
            role: TaskPoolWorkflowStatusRoleDto::Executing,
            enabled: true,
            terminal: false,
            next: None,
            transitions: Vec::new(),
        }],
    };
    let request = AgentSchedulerLegacyImportRequest {
        workflow: Some(workflow),
        dry_run: false,
        tasks: vec![AgentSchedulerLegacyTaskImportDto {
            task_id: "legacy-task-1".into(),
            title: "Imported legacy task".to_string(),
            status_key: Some("content_writing".to_string()),
            status: None,
            priority: 7,
            order_no: 3,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "Legacy prompt".to_string(),
            workflow_id: Some("content_flow".to_string()),
            metadata: json!({"task_board.status_role": "executing"}),
            created_at_ms: TimestampMs(1_783_000_930_000),
            updated_at_ms: Some(TimestampMs(1_783_000_930_500)),
        }],
    };
    let import_uri = format!("/agent-scheduler/projects/{project_id}/legacy-import");

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&import_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("import json")))
                .expect("import request"),
        )
        .await
        .expect("first import");
    assert_eq!(first.status(), StatusCode::OK);
    let body = response_json(first).await;
    assert_eq!(body["imported"], 1);
    assert_eq!(body["skipped"], 0);
    assert_eq!(body["tasks"][0]["status_key"], "content_writing");
    assert_eq!(body["tasks"][0]["read_model"]["workflow_id"], "content_flow");
    assert_eq!(body["events"][0]["kind"], "imported_legacy_task");

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&import_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("import json")))
                .expect("second import request"),
        )
        .await
        .expect("second import");
    assert_eq!(second.status(), StatusCode::OK);
    let body = response_json(second).await;
    assert_eq!(body["imported"], 0);
    assert_eq!(body["skipped"], 1);
    assert!(body["events"].as_array().expect("events").is_empty());

    let workflow =
        get_json(app.clone(), &format!("/agent-scheduler/projects/{project_id}/workflow")).await;
    assert_eq!(workflow["workflow"]["id"], "content_flow");
    let events = get_json(app, &format!("/agent-scheduler/projects/{project_id}/events")).await;
    assert_eq!(events["events"].as_array().expect("events").len(), 1);
}

#[tokio::test]
async fn agent_scheduler_gateway_local_command_returns_before_execution_finishes() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-nonblocking-command");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-gateway-blocking").expect("task id"),
                title: "Return before blocking local execution finishes".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-gateway-blocking/spec.md".to_string(),
                meta_uri: "shared/tasks/task-gateway-blocking/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Run this through a blocking local executor",
                    "priority": 3,
                    "order_no": 8,
                    "model": "auto",
                    "agent": "main"
                }),
                created_at_ms: TimestampMs(1_783_000_700_000),
            },
        )
        .expect("seed scheduler task");

    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(BlockingLocalExecutor {
        started: Mutex::new(Some(started_tx)),
        release: Mutex::new(release_rx),
        output: "released local execution".to_string(),
    });
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: "task-gateway-blocking".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("ready from gateway".to_string()),
        payload: Default::default(),
    };

    let response = tokio::time::timeout(
        Duration::from_secs(1),
        app.clone().oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        ),
    )
    .await
    .expect("local command should return while execution is still running")
    .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["task"]["task_id"], "task-gateway-blocking");
    assert_eq!(body["task"]["status"], "running");

    tokio::task::spawn_blocking(move || {
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("background execution should start after command returns")
    })
    .await
    .expect("started wait joins");

    let logs_uri =
        format!("/agent-scheduler/projects/{project_id}/tasks/task-gateway-blocking/logs");
    let running_logs = get_json(app.clone(), &logs_uri).await;
    assert!(
        running_logs["logs"]
            .as_array()
            .expect("logs array")
            .iter()
            .any(|log| log["message"] == "local execution started")
    );
    assert!(
        !running_logs["logs"]
            .as_array()
            .expect("logs array")
            .iter()
            .any(|log| log["message"] == "local execution submitted result")
    );

    release_tx.send(()).expect("release executor");
    let submitted_logs = wait_for_log(app, &logs_uri, "local execution submitted result").await;
    assert_eq!(submitted_logs["task_id"], "task-gateway-blocking");
}

#[tokio::test]
async fn agent_scheduler_gateway_start_is_idempotent_for_running_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-running-command");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-gateway-running").expect("task id"),
                title: "Already running gateway task".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Running,
                spec_uri: "shared/tasks/task-gateway-running/spec.md".to_string(),
                meta_uri: "shared/tasks/task-gateway-running/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "This task is already running",
                    "priority": 1,
                    "order_no": 2,
                    "model": "auto"
                }),
                created_at_ms: TimestampMs(1_783_000_600_000),
            },
        )
        .expect("seed running scheduler task");

    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "should not execute".to_string(),
    });
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: "task-gateway-running".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("duplicate start".to_string()),
        payload: Default::default(),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["task"]["task_id"], "task-gateway-running");
    assert_eq!(body["task"]["status"], "running");

    let task_id = TaskId::new("task-gateway-running").expect("task id");
    let task =
        store.load_task(&project_path, &task_id).expect("load scheduler task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Running);
}

#[tokio::test]
async fn agent_scheduler_merge_command_merges_submitted_task_to_completed() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-merge-command");
    init_merge_test_repo(&project_path, "feature/task-merge");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-merge-command").expect("task id"),
                title: "Merge submitted scheduler task".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Submitted,
                spec_uri: "shared/tasks/task-merge-command/spec.md".to_string(),
                meta_uri: "shared/tasks/task-merge-command/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "prompt": "Merge this submitted scheduler task",
                    "merge_source_branch": "feature/task-merge",
                    "merge_target_branch": "main",
                    "selected_worktree_path": project_path.to_string_lossy(),
                    "selected_worktree_branch": "feature/task-merge"
                }),
                created_at_ms: TimestampMs(1_783_000_800_000),
            },
        )
        .expect("seed submitted scheduler task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: "task-merge-command".into(),
        command: AgentSchedulerCommandKindDto::Merge,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("merge ready".to_string()),
        payload: Default::default(),
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(command_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&command).expect("command json")))
                .expect("command request"),
        )
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["task"]["task_id"], "task-merge-command");
    assert_eq!(body["task"]["status"], "completed");
    assert_eq!(body["event"]["payload"]["command"], "merge");

    let task_id = TaskId::new("task-merge-command").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(project_path.join("feature.txt")).expect("merged feature file"),
        "feature\n"
    );
    let status_uri = format!("/agent-scheduler/projects/{project_id}/tasks");
    let status_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(status_uri)
                .body(Body::empty())
                .expect("status request"),
        )
        .await
        .expect("status response");
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = response_json(status_response).await;
    assert!(
        status_body["tasks"].as_array().is_some_and(|tasks| {
            tasks.iter().any(|task| {
                task["task_id"] == "task-merge-command" && task["status"] == "completed"
            })
        }),
        "task board status should include completed merge task: {status_body}"
    );

    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert!(
        logs.iter().any(|log| log.message.contains("agent scheduler merge resolved:")),
        "merge diagnostics should include resolved branch/worktree context: {logs:?}"
    );
    assert!(
        logs.iter().any(|log| log.message.contains("agent scheduler merge git start:")),
        "merge diagnostics should include git start: {logs:?}"
    );
    assert!(
        logs.iter().any(|log| log.message.contains("agent scheduler merge lock released:")),
        "merge diagnostics should include lock release: {logs:?}"
    );
    assert!(logs.iter().any(|log| {
        log.from_status == Some(AgentTaskStatus::Submitted)
            && log.to_status == Some(AgentTaskStatus::Completed)
            && log.message.contains("agent scheduler merge completed")
    }));
    let artifacts = store.list_task_artifacts(&project_path, &task_id).expect("artifacts");
    assert!(artifacts.iter().any(|artifact| {
        artifact.kind == "merge_result" && artifact.payload["success"] == true
    }));
    let holder = super::super::task_pool::task_worktree_pool_service()
        .merge_lock_holder(&project_info, "main")
        .await
        .expect("merge lock holder");
    assert_eq!(holder, None);
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_merge_command_defaults_target_to_project_current_branch() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-merge-default-target");
    init_merge_test_repo(&project_path, "feature/task-merge-default-target");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    let mut task = submitted_merge_task(
        "task-merge-default-target",
        &project_path,
        "feature/task-merge-default-target",
        true,
    );
    task.metadata.as_object_mut().expect("metadata object").remove("merge_target_branch");
    store.create_task(&project_path, task).expect("seed submitted scheduler task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = post_merge_command(app, &project_id, "task-merge-default-target")
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let task_id = TaskId::new("task-merge-default-target").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(project_path.join("feature.txt")).expect("merged feature file"),
        "feature\n"
    );
    let artifacts = store.list_task_artifacts(&project_path, &task_id).expect("artifacts");
    assert!(artifacts.iter().any(|artifact| {
        artifact.kind == "merge_result"
            && artifact.payload["success"] == true
            && artifact.payload["target_branch"] == "main"
    }));
    let holder = super::super::task_pool::task_worktree_pool_service()
        .merge_lock_holder(&project_info, "main")
        .await
        .expect("merge lock holder");
    assert_eq!(holder, None);
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_merge_command_resolves_source_from_run_workspace() {
    use crate::agent_scheduler::store::logs::AgentSchedulerWorkspaceContext;
    use crate::agent_scheduler::store::runs::ClaimTaskRunRequest;

    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-merge-source-workspace");
    let source_branch = "feature/task-merge-source-workspace";
    init_merge_test_repo(&project_path, source_branch);
    let (project_id, project_info) = register_merge_test_project(&project_path).await;

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    let mut task =
        submitted_merge_task("task-merge-source-workspace", &project_path, source_branch, false);
    task.status = AgentTaskStatus::Running;
    task.metadata.as_object_mut().expect("metadata object").remove("merge_source_branch");
    let task_id = task.id.clone();
    store.create_task(&project_path, task).expect("seed running scheduler task");

    let run_id = RunId::new("run-merge-source-workspace").expect("run id");
    store
        .claim_task_run(
            &project_path,
            ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: WorkerName::new("gateway-local").expect("worker id"),
                lease_acquired_at_ms: TimestampMs(1_783_000_800_100),
                lease_expires_at_ms: None,
            },
        )
        .expect("claim run")
        .expect("claimed run");
    store
        .record_run_workspace_context(
            &project_path,
            &task_id,
            &run_id,
            AgentSchedulerWorkspaceContext {
                slot_id: "slot-source-workspace".to_string(),
                path: project_path.to_string_lossy().to_string(),
                branch: source_branch.to_string(),
            },
            TimestampMs(1_783_000_800_200),
        )
        .expect("record workspace")
        .expect("workspace recorded");
    store
        .transition_task_status(
            &project_path,
            &task_id,
            AgentTaskStatus::Submitted,
            TimestampMs(1_783_000_800_300),
        )
        .expect("transition submitted")
        .expect("submitted task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = post_merge_command(app, &project_id, "task-merge-source-workspace")
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::OK);

    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    let artifacts = store.list_task_artifacts(&project_path, &task_id).expect("artifacts");
    assert!(artifacts.iter().any(|artifact| {
        artifact.kind == "merge_result"
            && artifact.payload["success"] == true
            && artifact.payload["source_branch"] == source_branch
    }));
    let holder = super::super::task_pool::task_worktree_pool_service()
        .merge_lock_holder(&project_info, "main")
        .await
        .expect("merge lock holder");
    assert_eq!(holder, None);
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_merge_command_requires_worktree_metadata() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-merge-metadata");
    init_merge_test_repo(&project_path, "feature/task-merge-metadata");
    let (project_id, _project_info) = register_merge_test_project(&project_path).await;

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            submitted_merge_task(
                "task-merge-metadata",
                &project_path,
                "feature/task-merge-metadata",
                false,
            ),
        )
        .expect("seed submitted scheduler task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = post_merge_command(app, &project_id, "task-merge-metadata")
        .await
        .expect("command response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"], "selected_worktree_path is required");

    let task_id = TaskId::new("task-merge-metadata").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_gateway_merge_command_rejects_locked_target() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-merge-locked");
    init_merge_test_repo(&project_path, "feature/task-merge-locked");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;

    let holder_task_id = vw_api_types::id::TaskId("task-other-merge".to_string());
    let lock = super::super::task_pool::task_worktree_pool_service()
        .try_lock_merge_target(&project_info, &holder_task_id, "main")
        .await
        .expect("pre-lock merge target");
    assert!(lock.locked);

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            submitted_merge_task(
                "task-merge-locked",
                &project_path,
                "feature/task-merge-locked",
                true,
            ),
        )
        .expect("seed submitted scheduler task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response =
        post_merge_command(app, &project_id, "task-merge-locked").await.expect("command response");
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "unexpected response: {body}");
    assert!(body["error"].as_str().expect("error").contains("merge target locked"));

    let task_id = TaskId::new("task-merge-locked").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert!(
        logs.iter().any(|log| log.message.contains("agent scheduler merge blocked: target=main")),
        "merge lock conflict should be logged: {logs:?}"
    );
    let events = store.list_events_after(&project_path, None, 100).expect("events");
    assert!(events.iter().any(|event| {
        event.kind == AgentEventKind::TaskBlocked
            && event.payload["error"] == "merge_target_locked"
            && event.payload["holder_task_id"] == "task-other-merge"
    }));
    let holder = super::super::task_pool::task_worktree_pool_service()
        .merge_lock_holder(&project_info, "main")
        .await
        .expect("merge lock holder")
        .map(|holder| holder.0);
    assert_eq!(holder.as_deref(), Some("task-other-merge"));

    super::super::task_pool::task_worktree_pool_service()
        .unlock_merge_target(&project_info, &holder_task_id, "main")
        .await
        .expect("unlock pre-lock");
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_merge_command_releases_lock_after_git_failure() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-merge-failure");
    init_merge_test_repo(&project_path, "feature/task-merge-failure");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;

    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            submitted_merge_task(
                "task-merge-failure",
                &project_path,
                "feature/missing-merge-source",
                true,
            ),
        )
        .expect("seed submitted scheduler task");

    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response =
        post_merge_command(app, &project_id, "task-merge-failure").await.expect("command response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert!(body["error"].as_str().expect("error").contains("merge failed source="));

    let task_id = TaskId::new("task-merge-failure").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Failed);
    let holder = super::super::task_pool::task_worktree_pool_service()
        .merge_lock_holder(&project_info, "main")
        .await
        .expect("merge lock holder");
    assert_eq!(holder, None);
    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert!(logs.iter().any(|log| {
        log.from_status == Some(AgentTaskStatus::Submitted)
            && log.to_status == Some(AgentTaskStatus::Failed)
            && log.message.contains("merge failed source=feature/missing-merge-source")
    }));
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn gateway_local_executor_leases_task_worktree_instead_of_project_root() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-local-worktree");
    init_merge_test_repo(&project_path, "feature/local-worktree-base");
    let (project_id, project_info) = register_merge_test_project(&project_path).await;
    let task_id = TaskId::new("task-local-worktree").expect("task id");
    let request = LocalAgentWorkspaceRequest {
        project_path: project_path.clone(),
        task_id: task_id.clone(),
        worker_id: WorkerName::new(LOCAL_GATEWAY_WORKER_ID).expect("worker id"),
        run_id: RunId::new("run-local-worktree").expect("run id"),
    };

    let workspace = tokio::task::spawn_blocking(move || {
        GatewayLocalAgentExecutor::default().lease_workspace(request)
    })
    .await
    .expect("lease join")
    .expect("lease workspace");

    assert_ne!(PathBuf::from(&workspace.path), project_path);
    assert!(PathBuf::from(&workspace.path).is_dir());
    assert_eq!(workspace.branch, format!("vw/task/{}", task_id.as_str()));
    let snapshot = super::super::task_pool::task_worktree_pool_service()
        .snapshot(&project_info)
        .await
        .expect("snapshot");
    assert!(snapshot.slots.iter().any(|slot| {
        slot.path == workspace.path
            && slot.leased_task_id.as_ref().map(|task_id| task_id.as_ref())
                == Some(task_id.as_str())
    }));

    let api_task_id = vw_api_types::id::TaskId(task_id.as_str().to_string());
    super::super::task_pool::task_worktree_pool_service()
        .recycle_task_slot(&project_info, &api_task_id, Some("test cleanup".to_string()))
        .await
        .expect("cleanup slot");
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn agent_scheduler_gateway_upserts_project_worker_matrix_address() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-worker");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(StoreBackedAgentSchedulerGateway::default()))
        .with_state(());
    let workers_uri = format!("/agent-scheduler/projects/{project_id}/workers");
    let request = AgentSchedulerUpsertWorkerRequest {
        worker_id: "matrix-local".to_string(),
        runtime: Some("matrix".to_string()),
        deployment: Some("local".to_string()),
        model: Some("auto".to_string()),
        model_provider: None,
        status: Some(AgentWorkerStatusDto::Idle),
        desired_state: Some("Running".to_string()),
        team_id: None,
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@tester:localhost".to_string()),
        room_id: Some("!room:localhost".to_string()),
        container_state: Some("running".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(1_783_000_700_000)),
        last_heartbeat_at: None,
        capabilities: vec![AgentCapabilityDto {
            kind: AgentCapabilityKindDto::Skill,
            name: "matrix-dispatch".to_string(),
            metadata: json!({}),
        }],
        updated_at_ms: Some(TimestampMs(1_783_000_700_000)),
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&workers_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&request).expect("worker json")))
                .expect("worker request"),
        )
        .await
        .expect("worker response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["worker"]["id"], "matrix-local");
    assert_eq!(body["worker"]["room_id"], "!room:localhost");
    assert_eq!(body["worker"]["matrix_user_id"], "@tester:localhost");

    let list = get_json(app, &workers_uri).await;
    assert_eq!(list["workers"][0]["id"], "matrix-local");
    assert_eq!(list["workers"][0]["capabilities"][0]["name"], "matrix-dispatch");
}

#[tokio::test]
async fn agent_scheduler_gateway_manages_workers_and_teams_independently() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-management");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(StoreBackedAgentSchedulerGateway::default()))
        .with_state(());
    let workers_uri = format!("/agent-scheduler/projects/{project_id}/workers");
    let teams_uri = format!("/agent-scheduler/projects/{project_id}/teams");

    let worker = AgentSchedulerUpsertWorkerRequest {
        worker_id: "worker-management".to_string(),
        runtime: Some("codex".to_string()),
        deployment: Some("desktop".to_string()),
        model: Some("gpt-5".to_string()),
        model_provider: Some("openai".to_string()),
        status: Some(AgentWorkerStatusDto::Idle),
        desired_state: Some("running".to_string()),
        team_id: Some("team-management".to_string()),
        worker_role: Some("worker".to_string()),
        matrix_user_id: None,
        room_id: None,
        container_state: Some("running".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(1_783_001_200_000)),
        last_heartbeat_at: Some("2026-07-13T09:00:00Z".to_string()),
        capabilities: vec![AgentCapabilityDto {
            kind: AgentCapabilityKindDto::Skill,
            name: "gateway-management".to_string(),
            metadata: json!({}),
        }],
        updated_at_ms: Some(TimestampMs(1_783_001_200_100)),
    };
    let worker_response = post_json_response(app.clone(), &workers_uri, &worker).await;
    assert_eq!(worker_response.status(), StatusCode::OK);
    let worker_body = response_json(worker_response).await;
    assert_eq!(worker_body["worker"]["deployment"], "desktop");
    assert_eq!(worker_body["worker"]["last_seen_at_ms"], 1_783_001_200_000_i64);
    assert_eq!(worker_body["worker"]["updated_at_ms"], 1_783_001_200_100_i64);

    let leader = AgentTeamMemberDto {
        worker_id: "leader-management".to_string(),
        runtime_name: Some("leader-runtime".to_string()),
        role: "team_leader".to_string(),
        room_id: Some("!leader:example.test".to_string()),
        matrix_user_id: Some("@leader:example.test".to_string()),
        ready: true,
    };
    let member = AgentTeamMemberDto {
        worker_id: "worker-management".to_string(),
        runtime_name: Some("worker-runtime".to_string()),
        role: "worker".to_string(),
        room_id: None,
        matrix_user_id: None,
        ready: false,
    };
    let team = AgentSchedulerUpsertTeamRequest {
        team_id: "team-management".to_string(),
        name: "Management Team".to_string(),
        status: Some(AgentTeamStatusDto::Active),
        leader_id: "leader-management".to_string(),
        team_room_id: Some("!team:example.test".to_string()),
        leader_dm_room_id: Some("!leader-dm:example.test".to_string()),
        leader_ready: true,
        ready_workers: 1,
        total_workers: 2,
        members: vec![leader.clone(), member],
        updated_at_ms: Some(TimestampMs(1_783_001_300_000)),
    };
    let team_response = post_json_response(app.clone(), &teams_uri, &team).await;
    assert_eq!(team_response.status(), StatusCode::OK);
    let team_body = response_json(team_response).await;
    assert_eq!(team_body["team"]["id"], "team-management");
    assert_eq!(team_body["team"]["members"][0]["worker_id"], "leader-management");
    assert_eq!(team_body["team"]["updated_at_ms"], 1_783_001_300_000_i64);

    let workers = get_json(app.clone(), &workers_uri).await;
    assert_eq!(workers["workers"].as_array().expect("workers").len(), 1);
    assert_eq!(workers["workers"][0]["id"], "worker-management");
    let teams = get_json(app.clone(), &teams_uri).await;
    assert_eq!(teams["teams"].as_array().expect("teams").len(), 1);
    assert_eq!(teams["teams"][0]["id"], "team-management");

    let invalid_requests = [
        (
            AgentSchedulerUpsertTeamRequest { team_id: " ".to_string(), ..team.clone() },
            "team id must not be empty",
        ),
        (
            AgentSchedulerUpsertTeamRequest { name: " ".to_string(), ..team.clone() },
            "team name must not be empty",
        ),
        (
            AgentSchedulerUpsertTeamRequest { leader_id: " ".to_string(), ..team.clone() },
            "worker name must not be empty",
        ),
        (
            {
                let mut request = team.clone();
                request.members[1].worker_id = " ".to_string();
                request
            },
            "worker name must not be empty",
        ),
        (
            {
                let mut request = team.clone();
                request.members[1].role = " ".to_string();
                request
            },
            "team member role must not be empty",
        ),
        (
            AgentSchedulerUpsertTeamRequest {
                leader_id: "missing-leader".to_string(),
                ..team.clone()
            },
            "team leader_id must identify a team member",
        ),
        (
            AgentSchedulerUpsertTeamRequest { leader_ready: false, ..team.clone() },
            "team leader_ready must match the leader member ready state",
        ),
        (
            AgentSchedulerUpsertTeamRequest { ready_workers: 2, ..team.clone() },
            "team ready_workers must match ready member count 1",
        ),
        (
            AgentSchedulerUpsertTeamRequest { total_workers: 3, ..team },
            "team total_workers must match member count 2",
        ),
    ];
    for (request, expected_error) in invalid_requests {
        let response = post_json_response(app.clone(), &teams_uri, &request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = response_json(response).await["error"].as_str().expect("error").to_string();
        assert!(error.contains(expected_error), "unexpected error: {error}");
    }
}

#[test]
fn gateway_local_agent_stream_request_forwards_execution_selection() {
    let request = LocalAgentExecutionRequest {
        project_path: PathBuf::from("/tmp/project-root"),
        task_id: TaskId::new("task-deepseek").expect("task id"),
        worker_id: WorkerName::new("gateway-local").expect("worker id"),
        run_id: RunId::new("run-deepseek").expect("run id"),
        title: "Run with selected model".to_string(),
        prompt: "Use the selected model".to_string(),
        model: Some("deepseek/deepseek-v4-flash".to_string()),
        agent: None,
        acp_agent: Some("codex".to_string()),
        reasoning_effort: Some(vw_api_types::ReasoningEffortDto::High),
        workspace: crate::agent_scheduler::runner::local::LocalAgentWorkspace {
            slot_id: "slot-deepseek".to_string(),
            path: "/tmp/project-root/worktrees/slot-deepseek".to_string(),
            branch: "codex/task-deepseek".to_string(),
        },
    };

    let body = gateway_local_agent_stream_request(&request);

    assert_eq!(body.model.as_deref(), Some("deepseek/deepseek-v4-flash"));
    assert_eq!(body.agent, None);
    assert_eq!(body.acp_agent.as_deref(), Some("codex"));
    assert_eq!(body.reasoning_effort, Some(vw_api_types::ReasoningEffortDto::High));
    assert_eq!(body.messages[0]["content"], "Use the selected model");
    let options = body.options.expect("gateway options");
    assert_eq!(options["cwd"], "/tmp/project-root/worktrees/slot-deepseek");
    assert_eq!(options["acp_test"], true);
    assert_eq!(options["acp_agent"], "codex");
    assert_eq!(options["acp_permission_mode"], "approve-all");
    assert_eq!(options["acp_force_new_session"], true);
    assert_eq!(options["acp_close_client_after_request"], true);
    assert_eq!(options["acp_history_strategy"], "discard");
    assert_eq!(options["acp_history_recent_count"], 1);
}

#[test]
fn gateway_local_agent_stream_payloads_append_task_logs() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-stream-logs");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).expect("init scheduler");
    let task_id = TaskId::new("task-stream-logs").expect("task id");
    let run_id = RunId::new("run-stream-logs").expect("run id");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: task_id.clone(),
                title: "Record gateway stream logs".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Running,
                spec_uri: "shared/tasks/task-stream-logs/spec.md".to_string(),
                meta_uri: "shared/tasks/task-stream-logs/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({"prompt": "record stream logs"}),
                created_at_ms: TimestampMs(1_783_000_950_000),
            },
        )
        .expect("seed stream task");
    store
        .claim_task_run(
            &project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: run_id.clone(),
                task_id: task_id.clone(),
                worker_id: WorkerName::new(LOCAL_GATEWAY_WORKER_ID).expect("worker id"),
                lease_acquired_at_ms: TimestampMs(1_783_000_951_000),
                lease_expires_at_ms: Some(TimestampMs(1_783_001_951_000)),
            },
        )
        .expect("claim run")
        .expect("run claimed");

    let long_delta = format!("visible output {}", "x".repeat(600));
    for payload in [
        json!({"type": "chat.step_start", "step_index": 1, "model": "test/model"}),
        json!({"type": "chat.delta", "delta": "   \n\t"}),
        json!({"type": "chat.delta", "delta": long_delta}),
        json!({"type": "chat.post_tool_round", "step_index": 1}),
        json!({"type": "chat.step_finish", "step_index": 1, "finish_reason": "stop"}),
        json!({"type": "chat.error", "error": "synthetic gateway failure"}),
        json!({"type": "chat.done", "message_id": "assistant-msg"}),
    ] {
        append_gateway_stream_payload_task_log(
            &store,
            &project_path,
            &task_id,
            &run_id,
            TimestampMs(1_783_000_952_000),
            &payload,
        )
        .expect("append stream payload log");
    }

    let logs = store.list_task_logs(&project_path, &task_id).expect("logs");
    assert_eq!(logs.len(), 6);
    assert!(logs.iter().all(|log| log.run_id.as_ref() == Some(&run_id)));
    assert!(logs.iter().any(|log| log.message == "gateway stream step 1 started model=test/model"));
    let delta_log = logs
        .iter()
        .find(|log| log.message.starts_with("gateway stream output: visible output"))
        .expect("delta log");
    assert!(delta_log.message.len() < 360);
    assert!(delta_log.message.ends_with("..."));
    assert!(logs.iter().any(|log| log.message == "gateway stream step 1 completed tool round"));
    assert!(
        logs.iter()
            .any(|log| { log.message == "gateway stream step 1 finished finish_reason=stop" })
    );
    assert!(
        logs.iter().any(|log| { log.message == "gateway stream error: synthetic gateway failure" })
    );
    assert!(logs.iter().any(|log| log.message == "gateway stream completed"));
}

#[test]
fn scheduler_enum_mapping_helpers_cover_all_variants() {
    for (dto, domain) in [
        (AgentTaskKindDto::Finite, AgentTaskKind::Finite),
        (AgentTaskKindDto::Infinite, AgentTaskKind::Infinite),
        (AgentTaskKindDto::TeamDag, AgentTaskKind::TeamDag),
    ] {
        assert_eq!(task_kind_from_dto(dto.clone()), domain);
        assert_eq!(task_kind_to_dto(domain), dto);
    }

    for (dto, domain) in [
        (AgentTaskStatusDto::Draft, AgentTaskStatus::Draft),
        (AgentTaskStatusDto::Queued, AgentTaskStatus::Queued),
        (AgentTaskStatusDto::Assigned, AgentTaskStatus::Assigned),
        (AgentTaskStatusDto::Dispatched, AgentTaskStatus::Dispatched),
        (AgentTaskStatusDto::Acknowledged, AgentTaskStatus::Acknowledged),
        (AgentTaskStatusDto::Active, AgentTaskStatus::Active),
        (AgentTaskStatusDto::Running, AgentTaskStatus::Running),
        (AgentTaskStatusDto::Submitted, AgentTaskStatus::Submitted),
        (AgentTaskStatusDto::Revision, AgentTaskStatus::Revision),
        (AgentTaskStatusDto::Blocked, AgentTaskStatus::Blocked),
        (AgentTaskStatusDto::Paused, AgentTaskStatus::Paused),
        (AgentTaskStatusDto::Completed, AgentTaskStatus::Completed),
        (AgentTaskStatusDto::Failed, AgentTaskStatus::Failed),
        (AgentTaskStatusDto::Cancelled, AgentTaskStatus::Cancelled),
        (AgentTaskStatusDto::Archived, AgentTaskStatus::Archived),
    ] {
        assert_eq!(task_status_from_dto(dto.clone()), domain);
        assert_eq!(task_status_to_dto(domain), dto);
    }

    for (dto, domain) in [
        (AgentWorkerStatusDto::Pending, AgentWorkerStatus::Pending),
        (AgentWorkerStatusDto::Idle, AgentWorkerStatus::Idle),
        (AgentWorkerStatusDto::Busy, AgentWorkerStatus::Busy),
        (AgentWorkerStatusDto::Running, AgentWorkerStatus::Running),
        (AgentWorkerStatusDto::Sleeping, AgentWorkerStatus::Sleeping),
        (AgentWorkerStatusDto::Stopped, AgentWorkerStatus::Stopped),
        (AgentWorkerStatusDto::Failed, AgentWorkerStatus::Failed),
        (AgentWorkerStatusDto::Unavailable, AgentWorkerStatus::Unavailable),
    ] {
        assert_eq!(worker_status_from_dto(dto.clone()), domain);
        assert_eq!(worker_status_to_dto(domain), dto);
    }

    for (dto, domain) in [
        (AgentCapabilityKindDto::Skill, AgentCapabilityKind::Skill),
        (AgentCapabilityKindDto::RemoteSkill, AgentCapabilityKind::RemoteSkill),
        (AgentCapabilityKindDto::McpServer, AgentCapabilityKind::McpServer),
        (AgentCapabilityKindDto::Model, AgentCapabilityKind::Model),
        (AgentCapabilityKindDto::Runtime, AgentCapabilityKind::Runtime),
        (AgentCapabilityKindDto::Tool, AgentCapabilityKind::Tool),
    ] {
        assert_eq!(capability_kind_from_dto(dto.clone()), domain);
        assert_eq!(capability_kind_to_dto(domain), dto);
    }

    for (dto, domain) in [
        (AgentTeamStatusDto::Pending, AgentTeamStatus::Pending),
        (AgentTeamStatusDto::Active, AgentTeamStatus::Active),
        (AgentTeamStatusDto::Degraded, AgentTeamStatus::Degraded),
        (AgentTeamStatusDto::Failed, AgentTeamStatus::Failed),
    ] {
        assert_eq!(team_status_from_dto(dto.clone()), domain);
        assert_eq!(team_status_to_dto(domain), dto);
    }

    for (dto, domain) in [
        (AgentRunStatusDto::Pending, AgentRunStatus::Pending),
        (AgentRunStatusDto::Leased, AgentRunStatus::Leased),
        (AgentRunStatusDto::Running, AgentRunStatus::Running),
        (AgentRunStatusDto::Succeeded, AgentRunStatus::Succeeded),
        (AgentRunStatusDto::Failed, AgentRunStatus::Failed),
        (AgentRunStatusDto::Cancelled, AgentRunStatus::Cancelled),
        (AgentRunStatusDto::TimedOut, AgentRunStatus::TimedOut),
        (AgentRunStatusDto::Stale, AgentRunStatus::Stale),
    ] {
        assert_eq!(run_status_to_dto(domain), dto);
    }

    for (status, stage) in [
        (AgentRunStatus::Pending, Some("planning")),
        (AgentRunStatus::Leased, Some("running")),
        (AgentRunStatus::Running, Some("running")),
        (AgentRunStatus::Succeeded, Some("succeeded")),
        (AgentRunStatus::Failed, Some("failed")),
        (AgentRunStatus::Cancelled, Some("canceled")),
        (AgentRunStatus::TimedOut, Some("failed")),
        (AgentRunStatus::Stale, Some("failed")),
    ] {
        assert_eq!(run_stage_from_run_status(status), stage);
    }

    assert_eq!(run_stage_from_task_status(AgentTaskStatus::Queued), None);
    assert_eq!(run_stage_from_task_status(AgentTaskStatus::Running), Some("running"));
    assert_eq!(run_stage_from_task_status(AgentTaskStatus::Completed), Some("succeeded"));
    assert_eq!(run_stage_from_task_status(AgentTaskStatus::Failed), Some("failed"));
    assert_eq!(run_stage_from_task_status(AgentTaskStatus::Archived), Some("canceled"));
    assert_eq!(
        outbox_status_to_dto(OutboxMessageStatus::Pending),
        AgentOutboxMessageStatusDto::Pending
    );
    assert_eq!(
        outbox_status_to_dto(OutboxMessageStatus::Claimed),
        AgentOutboxMessageStatusDto::Sending
    );
    assert_eq!(
        outbox_status_to_dto(OutboxMessageStatus::Failed),
        AgentOutboxMessageStatusDto::Failed
    );
    assert_eq!(
        outbox_status_to_dto(OutboxMessageStatus::Delivered),
        AgentOutboxMessageStatusDto::Delivered
    );
    assert_eq!(dispatch_mode_from_dto(AgentSchedulerDispatchModeDto::Local), DispatchMode::Local);
    assert_eq!(dispatch_mode_from_dto(AgentSchedulerDispatchModeDto::Matrix), DispatchMode::Matrix);
    assert_eq!(dispatch_mode_from_dto(AgentSchedulerDispatchModeDto::Auto), DispatchMode::Auto);

    for (kind, dto) in [
        (AgentEventKind::TaskCreated, AgentEventKindDto::TaskCreated),
        (AgentEventKind::ImportedLegacyTask, AgentEventKindDto::ImportedLegacyTask),
        (AgentEventKind::TaskAssigned, AgentEventKindDto::TaskAssigned),
        (AgentEventKind::WorkerSelected, AgentEventKindDto::WorkerSelected),
        (AgentEventKind::MatrixDispatched, AgentEventKindDto::MatrixDispatched),
        (AgentEventKind::WorkerAcknowledged, AgentEventKindDto::WorkerAcknowledged),
        (AgentEventKind::RunStarted, AgentEventKindDto::RunStarted),
        (AgentEventKind::RunSubmitted, AgentEventKindDto::RunSubmitted),
        (AgentEventKind::RunCompleted, AgentEventKindDto::RunCompleted),
        (AgentEventKind::RunFailed, AgentEventKindDto::RunFailed),
        (AgentEventKind::RunRecovered, AgentEventKindDto::RunRecovered),
        (AgentEventKind::TaskBlocked, AgentEventKindDto::TaskBlocked),
        (AgentEventKind::TaskPaused, AgentEventKindDto::TaskPaused),
        (AgentEventKind::TaskResumed, AgentEventKindDto::TaskResumed),
        (AgentEventKind::TaskCancelled, AgentEventKindDto::TaskCancelled),
        (AgentEventKind::TaskArchived, AgentEventKindDto::TaskArchived),
        (AgentEventKind::InfiniteScheduled, AgentEventKindDto::InfiniteScheduled),
        (AgentEventKind::InfiniteExecuted, AgentEventKindDto::InfiniteExecuted),
        (AgentEventKind::TeamNodeQueued, AgentEventKindDto::TeamNodeQueued),
        (AgentEventKind::TeamNodeDispatched, AgentEventKindDto::TeamNodeDispatched),
        (AgentEventKind::TeamNodeCompleted, AgentEventKindDto::TeamNodeCompleted),
        (AgentEventKind::ResourceLeased, AgentEventKindDto::ResourceLeased),
        (AgentEventKind::ResourceReleased, AgentEventKindDto::ResourceReleased),
        (AgentEventKind::OutboxQueued, AgentEventKindDto::OutboxQueued),
        (AgentEventKind::OutboxDelivered, AgentEventKindDto::OutboxDelivered),
        (AgentEventKind::OutboxFailed, AgentEventKindDto::OutboxFailed),
        (AgentEventKind::HumanIntervened, AgentEventKindDto::HumanIntervened),
    ] {
        assert_eq!(event_kind_to_dto(kind), dto);
    }
}

#[test]
fn scheduler_command_and_metadata_helpers_cover_edge_cases() {
    for (command, key, event_kind, is_board_write) in [
        (AgentSchedulerCommandKindDto::Start, "start", AgentEventKind::HumanIntervened, false),
        (AgentSchedulerCommandKindDto::Retry, "retry", AgentEventKind::HumanIntervened, false),
        (AgentSchedulerCommandKindDto::Pause, "pause", AgentEventKind::TaskPaused, false),
        (AgentSchedulerCommandKindDto::Resume, "resume", AgentEventKind::TaskResumed, false),
        (AgentSchedulerCommandKindDto::Cancel, "cancel", AgentEventKind::TaskCancelled, false),
        (AgentSchedulerCommandKindDto::Archive, "archive", AgentEventKind::TaskArchived, true),
        (AgentSchedulerCommandKindDto::Promote, "promote", AgentEventKind::HumanIntervened, false),
        (AgentSchedulerCommandKindDto::Merge, "merge", AgentEventKind::HumanIntervened, false),
        (AgentSchedulerCommandKindDto::Move, "move", AgentEventKind::HumanIntervened, true),
        (
            AgentSchedulerCommandKindDto::UpdateMetadata,
            "update_metadata",
            AgentEventKind::HumanIntervened,
            true,
        ),
        (
            AgentSchedulerCommandKindDto::SoftDelete,
            "soft_delete",
            AgentEventKind::HumanIntervened,
            true,
        ),
        (AgentSchedulerCommandKindDto::Restore, "restore", AgentEventKind::HumanIntervened, true),
        (
            AgentSchedulerCommandKindDto::ImportLegacyTask,
            "import_legacy_task",
            AgentEventKind::HumanIntervened,
            true,
        ),
        (
            AgentSchedulerCommandKindDto::AssignStaff,
            "assign_staff",
            AgentEventKind::HumanIntervened,
            true,
        ),
        (
            AgentSchedulerCommandKindDto::RunStageAction,
            "run_stage_action",
            AgentEventKind::HumanIntervened,
            true,
        ),
    ] {
        assert_eq!(command_kind_key(command), key);
        assert_eq!(command_event_kind(command), event_kind);
        assert_eq!(task_board_write_command(command), is_board_write);
    }

    for (role, key) in [
        (TaskPoolWorkflowStatusRoleDto::Backlog, "backlog"),
        (TaskPoolWorkflowStatusRoleDto::Queued, "queued"),
        (TaskPoolWorkflowStatusRoleDto::Planning, "planning"),
        (TaskPoolWorkflowStatusRoleDto::Executing, "executing"),
        (TaskPoolWorkflowStatusRoleDto::Failed, "failed"),
        (TaskPoolWorkflowStatusRoleDto::Paused, "paused"),
        (TaskPoolWorkflowStatusRoleDto::ReviewReady, "review_ready"),
        (TaskPoolWorkflowStatusRoleDto::Review, "review"),
        (TaskPoolWorkflowStatusRoleDto::Merging, "merging"),
        (TaskPoolWorkflowStatusRoleDto::Done, "done"),
        (TaskPoolWorkflowStatusRoleDto::Archived, "archived"),
        (TaskPoolWorkflowStatusRoleDto::Custom, "custom"),
    ] {
        assert_eq!(
            task_board_status_role_from_metadata(&json!({"task_board.status_role": key})),
            Some(role)
        );
    }
    assert_eq!(
        task_board_status_role_from_metadata(&json!({"task_board.status_role": "nope"})),
        None
    );

    assert_eq!(task_board_status_key(AgentTaskStatus::Submitted, &json!({})), "code_complete");
    assert_eq!(
        task_board_status_key(
            AgentTaskStatus::Running,
            &json!({"task_board.status_key_map": {"running": "doing"}})
        ),
        "doing"
    );
    assert_eq!(legacy_task_board_status_key(AgentTaskStatus::Queued), "queued");
    assert!(valid_merge_branch("feature/demo"));
    assert!(!valid_merge_branch(" "));
    assert!(!valid_merge_branch("HEAD"));
    assert_eq!(non_empty_str("  value  "), Some("value"));
    assert_eq!(optional_text(Some("  model  ".to_string())), Some("model".to_string()));
    assert_eq!(optional_text(Some("   ".to_string())), None);
    assert_eq!(
        parse_project_path("  /tmp/project  ").expect("path"),
        PathBuf::from("/tmp/project")
    );
    assert_eq!(
        parse_project_path("  ").expect_err("empty project path").to_string(),
        "project_path must not be empty"
    );
    assert_eq!(map_bad_request(anyhow::anyhow!("bad")).to_string(), "bad");
    assert_eq!(
        map_scheduler_command_error(anyhow::anyhow!("unsupported agent task transition: bad"))
            .to_string(),
        "unsupported agent task transition: bad"
    );
    assert_eq!(map_task_board_error(anyhow::anyhow!("board bad")).to_string(), "board bad");
}

#[test]
fn scheduler_request_builders_validate_and_trim_inputs() {
    let task = new_task_from_request(AgentSchedulerCreateTaskRequest {
        task_id: "task-builder".to_string(),
        title: "Build task".to_string(),
        kind: AgentTaskKindDto::Infinite,
        status: None,
        spec_uri: "spec.md".to_string(),
        meta_uri: "meta.json".to_string(),
        assigned_to: Some("worker-a".to_string()),
        team_id: Some("team-a".to_string()),
        room_id: Some("!room:example.test".to_string()),
        project_room_id: Some("!project:example.test".to_string()),
        delegated_to_team: Some("team-b".to_string()),
        schedule: Some("@hourly".to_string()),
        timezone: Some("UTC".to_string()),
        next_scheduled_at: Some("2026-01-01T00:00:00Z".to_string()),
        last_executed_at: Some("2025-12-31T23:00:00Z".to_string()),
        metadata: json!({"prompt": "do it"}),
        created_at_ms: TimestampMs(10),
    })
    .expect("task request");
    assert_eq!(task.status, AgentTaskStatus::Queued);
    assert_eq!(task.kind, AgentTaskKind::Infinite);
    assert_eq!(task.assigned_to.as_ref().map(WorkerName::as_str), Some("worker-a"));
    assert!(
        new_task_from_request(AgentSchedulerCreateTaskRequest {
            task_id: "task-empty-title".to_string(),
            title: " ".to_string(),
            kind: AgentTaskKindDto::Finite,
            status: Some(AgentTaskStatusDto::Draft),
            spec_uri: "spec.md".to_string(),
            meta_uri: "meta.json".to_string(),
            assigned_to: None,
            team_id: None,
            room_id: None,
            project_room_id: None,
            delegated_to_team: None,
            schedule: None,
            timezone: None,
            next_scheduled_at: None,
            last_executed_at: None,
            metadata: json!({}),
            created_at_ms: TimestampMs(11),
        })
        .expect_err("empty title")
        .to_string()
        .contains("task title must not be empty")
    );

    let worker = new_worker_from_request(
        AgentSchedulerUpsertWorkerRequest {
            worker_id: "worker-builder".to_string(),
            runtime: Some("  local  ".to_string()),
            deployment: Some(" ".to_string()),
            model: Some(" ".to_string()),
            model_provider: Some("openai".to_string()),
            status: Some(AgentWorkerStatusDto::Busy),
            desired_state: Some("Sleeping".to_string()),
            team_id: Some("team-builder".to_string()),
            worker_role: Some(" reviewer ".to_string()),
            matrix_user_id: Some("@worker:example.test".to_string()),
            room_id: Some("!room:example.test".to_string()),
            container_state: Some("running".to_string()),
            idle_since_ms: Some(TimestampMs(12)),
            last_seen_at_ms: Some(TimestampMs(13)),
            last_heartbeat_at: Some(" heartbeat ".to_string()),
            capabilities: vec![AgentCapabilityDto {
                kind: AgentCapabilityKindDto::Tool,
                name: " shell ".to_string(),
                metadata: json!({"safe": true}),
            }],
            updated_at_ms: None,
        },
        TimestampMs(14),
    )
    .expect("worker request");
    assert_eq!(worker.runtime.as_deref(), Some("local"));
    assert_eq!(worker.deployment, None);
    assert_eq!(worker.model, "auto");
    assert_eq!(worker.worker_role.as_deref(), Some("reviewer"));
    assert_eq!(worker.last_heartbeat_at.as_deref(), Some("heartbeat"));
    assert_eq!(worker.capabilities[0].kind, AgentCapabilityKind::Tool);
    assert_eq!(worker.capabilities[0].name, " shell ");
    assert_eq!(worker.updated_at_ms, TimestampMs(14));
    assert_eq!(
        capability_from_dto(AgentCapabilityDto {
            kind: AgentCapabilityKindDto::Skill,
            name: " ".to_string(),
            metadata: json!({}),
        })
        .expect_err("empty capability name")
        .to_string(),
        "worker capability name must not be empty"
    );

    let member = AgentTeamMemberDto {
        worker_id: "leader".to_string(),
        runtime_name: Some("runtime-leader".to_string()),
        role: " lead ".to_string(),
        room_id: Some("!leader:example.test".to_string()),
        matrix_user_id: Some("@leader:example.test".to_string()),
        ready: true,
    };
    let team_request = AgentSchedulerUpsertTeamRequest {
        team_id: "team-builder".to_string(),
        name: " Builders ".to_string(),
        status: Some(AgentTeamStatusDto::Active),
        leader_id: "leader".to_string(),
        team_room_id: Some("!team:example.test".to_string()),
        leader_dm_room_id: Some("!dm:example.test".to_string()),
        leader_ready: true,
        ready_workers: 1,
        total_workers: 1,
        members: vec![member],
        updated_at_ms: None,
    };
    let team = new_team_from_request(team_request.clone(), TimestampMs(15)).expect("team request");
    assert_eq!(team.name, " Builders ");
    assert_eq!(team.status, AgentTeamStatus::Active);
    assert_eq!(team.members[0].role, " lead ");
    assert_eq!(
        new_team_from_request(
            AgentSchedulerUpsertTeamRequest { leader_ready: false, ..team_request.clone() },
            TimestampMs(15),
        )
        .expect_err("leader readiness mismatch")
        .to_string(),
        "team leader_ready must match the leader member ready state"
    );
    assert_eq!(
        new_team_from_request(
            AgentSchedulerUpsertTeamRequest { total_workers: 2, ..team_request.clone() },
            TimestampMs(15),
        )
        .expect_err("total mismatch")
        .to_string(),
        "team total_workers must match member count 1"
    );
    assert_eq!(
        new_team_from_request(
            AgentSchedulerUpsertTeamRequest { ready_workers: 0, ..team_request.clone() },
            TimestampMs(15),
        )
        .expect_err("ready mismatch")
        .to_string(),
        "team ready_workers must match ready member count 1"
    );
    assert_eq!(
        new_team_from_request(
            AgentSchedulerUpsertTeamRequest { leader_id: "missing".to_string(), ..team_request },
            TimestampMs(15),
        )
        .expect_err("missing leader")
        .to_string(),
        "team leader_id must identify a team member"
    );
}

#[test]
fn scheduler_dto_helpers_preserve_metadata_workspace_and_errors() {
    let task_id = TaskId::new("task-dto").expect("task id");
    let run_id = RunId::new("run-dto").expect("run id");
    let worker_id = WorkerName::new("worker-dto").expect("worker id");
    let team_id = TeamId::new("team-dto").expect("team id");
    let task = StoredAgentTask {
        id: task_id.clone(),
        title: "DTO task".to_string(),
        kind: AgentTaskKind::TeamDag,
        status: AgentTaskStatus::Submitted,
        spec_uri: "spec.md".to_string(),
        meta_uri: "meta.json".to_string(),
        assigned_to: Some(worker_id.clone()),
        team_id: Some(team_id.clone()),
        room_id: Some("!room:example.test".to_string()),
        project_room_id: Some("!project:example.test".to_string()),
        delegated_to_team: Some(team_id.clone()),
        schedule: Some("@daily".to_string()),
        timezone: Some("UTC".to_string()),
        next_scheduled_at: Some("next".to_string()),
        last_executed_at: Some("last".to_string()),
        dag: vec![AgentTaskDagNode {
            task_id: TaskId::new("task-dep").expect("dep id"),
            title: "Dependency".to_string(),
            depends_on: vec![TaskId::new("task-root").expect("root id")],
            assigned_to: Some(worker_id.clone()),
            status: AgentTaskStatus::Completed,
        }],
        metadata: json!({
            "priority": 8,
            "order_no": -2,
            "model": "gpt-test",
            "agent": "main",
            "acp_agent": "codex",
            "reasoning_effort": "high",
            "prompt": "custom prompt",
            "pause_reason": "waiting",
            "last_error": "metadata error",
            "task_board.workflow_id": "wf",
            "task_board.status_label": "Review",
            "task_board.status_role": "review_ready",
            "task_board.stage_id": "review",
            "task_board.stage_label": "Review Stage",
            "task_board.staff.member_id": "alice",
            "task_board.staff.member_role": "reviewer",
            "task_board.staff.primary_member_id": "alice",
            "task_board.staff.backup_member_ids": ["bob", "", 7],
            "task_board.staff.manager_member_ids": ["lead"],
            "task_board.staff.autonomous": true,
            "task_board.stage_action.capability_id": "builtin.review",
            "task_board.stage_action.workflow_uuid": "workflow-uuid",
            "workspace": {
                "slot_id": "slot-run",
                "path": "/tmp/workspace",
                "branch": "feature/dto"
            }
        }),
        created_at_ms: TimestampMs(20),
        updated_at_ms: TimestampMs(21),
    };
    let run = StoredAgentRun {
        id: run_id.clone(),
        task_id: task_id.clone(),
        worker_id: Some(worker_id.clone()),
        status: AgentRunStatus::Failed,
        attempt: 3,
        lease_acquired_at_ms: Some(TimestampMs(22)),
        lease_expires_at_ms: Some(TimestampMs(23)),
        started_at_ms: Some(TimestampMs(24)),
        finished_at_ms: Some(TimestampMs(25)),
        last_heartbeat_at_ms: Some(TimestampMs(26)),
        error_message: Some("run error".to_string()),
    };
    let store = AgentSchedulerStore::new();
    let temp = tempfile::tempdir().expect("temp project");
    let project_path = temp.path().join("dto-project");
    std::fs::create_dir_all(&project_path).expect("project dir");
    store.initialize_project(&project_path).expect("init scheduler");

    let board_task =
        task_board_task_to_dto(&store, "project-dto", &project_path, task.clone(), Some(&run))
            .expect("board task dto");
    assert_eq!(board_task.project_id.as_ref(), "project-dto");
    assert_eq!(board_task.status_key, "code_complete");
    assert_eq!(board_task.retry_count, 2);
    assert_eq!(board_task.run_stage.as_deref(), Some("failed"));
    assert_eq!(board_task.last_error.as_deref(), Some("run error"));
    assert_eq!(board_task.merge_source_branch.as_deref(), Some("feature/dto"));
    assert_eq!(board_task.workspace.as_ref().expect("workspace").slot_id, "slot-run");
    assert_eq!(board_task.read_model.staff.as_ref().expect("staff").backup_member_ids, vec!["bob"]);
    assert!(!board_task.read_model.available_stage_actions.is_empty());

    let task_dto = task_to_dto(task.clone());
    assert_eq!(task_dto.dag[0].depends_on, vec!["task-root"]);
    let run_dto = run_to_dto(run.clone());
    assert_eq!(run_dto.worker_id.as_deref(), Some("worker-dto"));
    assert_eq!(run_dto.status, AgentRunStatusDto::Failed);

    let log = task_log_to_dto(StoredAgentTaskLog {
        task_id: task_id.clone(),
        run_id: Some(run_id.clone()),
        seq_no: 7,
        timestamp_ms: TimestampMs(27),
        from_status: Some(AgentTaskStatus::Queued),
        to_status: Some(AgentTaskStatus::Running),
        from_status_key: None,
        to_status_key: Some("doing".to_string()),
        message: "log message".to_string(),
    });
    assert_eq!(log.from_status_key.as_deref(), Some("queued"));
    assert_eq!(log.to_status_key.as_deref(), Some("doing"));

    let artifact = task_artifact_to_dto(StoredTaskArtifact {
        id: 1,
        task_id: task_id.clone(),
        run_id: Some(run_id.clone()),
        kind: "merge_result".to_string(),
        name: "merge-result.json".to_string(),
        path: Some("artifact.json".to_string()),
        payload: json!({"ok": true}),
        created_at_ms: TimestampMs(28),
    });
    assert_eq!(artifact.run_id.as_deref(), Some("run-dto"));

    let outbox = outbox_to_dto(StoredOutboxMessage {
        id: "outbox-dto".to_string(),
        channel: "matrix".to_string(),
        target: "!room:example.test".to_string(),
        recipient_id: Some("@worker:example.test".to_string()),
        body: "body".to_string(),
        status: OutboxMessageStatus::Delivered,
        idempotency_key: "idem".to_string(),
        attempts: 2,
        next_attempt_at_ms: Some(TimestampMs(30)),
        delivered_at_ms: Some(TimestampMs(31)),
        last_error: Some("old error".to_string()),
        payload: json!({"task_id": "task-dto"}),
        created_at_ms: TimestampMs(32),
        updated_at_ms: TimestampMs(33),
    });
    assert_eq!(outbox.id, "outbox-dto");
    assert_eq!(outbox.status, AgentOutboxMessageStatusDto::Delivered);

    let event = StoredAgentEvent {
        cursor: 9,
        task_id: None,
        run_id: Some(run_id.clone()),
        worker_id: Some(worker_id.clone()),
        team_id: Some(team_id.clone()),
        kind: AgentEventKind::RunFailed,
        from_status: Some(AgentTaskStatus::Running),
        to_status: Some(AgentTaskStatus::Failed),
        from_status_key: None,
        to_status_key: None,
        message: "failed from event".to_string(),
        payload: json!({
            "task_id": "task-dto",
            "error": "payload error",
            "selected_worktree_slot_id": "slot-selected",
            "selected_worktree_path": "/tmp/selected",
            "selected_worktree_branch": "feature/selected"
        }),
        created_at_ms: TimestampMs(29),
    };
    let generic_event = event_to_dto(event.clone());
    assert_eq!(generic_event.id, "9");
    assert_eq!(generic_event.kind, AgentEventKindDto::RunFailed);
    let board_event = task_board_event_to_dto("project-dto", event).expect("board event dto");
    assert_eq!(board_event.task_id.as_ref(), "task-dto");
    assert_eq!(board_event.error_message.as_deref(), Some("payload error"));
    assert_eq!(board_event.workspace.as_ref().expect("event workspace").branch, "feature/selected");
    assert_eq!(
        task_board_event_to_dto(
            "project-dto",
            StoredAgentEvent {
                cursor: 10,
                task_id: None,
                run_id: None,
                worker_id: None,
                team_id: None,
                kind: AgentEventKind::HumanIntervened,
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message: "missing task".to_string(),
                payload: json!({}),
                created_at_ms: TimestampMs(30),
            },
        )
        .expect_err("task id required")
        .to_string(),
        "agent scheduler task board event requires task id"
    );
}

#[test]
fn scheduler_response_helpers_preserve_outcomes_and_payloads() {
    let task_id = TaskId::new("task-response").expect("task id");
    let run_id = RunId::new("run-response").expect("run id");
    let worker_id = WorkerName::new("worker-response").expect("worker id");
    let run = StoredAgentRun {
        id: run_id.clone(),
        task_id: task_id.clone(),
        worker_id: Some(worker_id.clone()),
        status: AgentRunStatus::Running,
        attempt: 1,
        lease_acquired_at_ms: None,
        lease_expires_at_ms: None,
        started_at_ms: Some(TimestampMs(31)),
        finished_at_ms: None,
        last_heartbeat_at_ms: None,
        error_message: None,
    };

    let local = project_wake_response_from_local_outcome(Some(LocalAgentRunnerOutcome {
        task_id: task_id.clone(),
        worker_id: worker_id.clone(),
        run: run.clone(),
    }));
    assert!(local.dispatched);
    assert_eq!(local.run_id.as_deref(), Some("run-response"));
    assert!(!project_wake_response_from_local_outcome(None).dispatched);

    let dispatch = crate::agent_scheduler::engine::dispatch::DispatchOutcome {
        task_id: task_id.clone(),
        run_id: run_id.clone(),
        worker_id: worker_id.clone(),
        target: crate::agent_scheduler::engine::dispatch::DispatchTarget::Matrix(
            crate::agent_scheduler::adapters::matrix::MatrixDispatchTarget {
                room_id: "!room:example.test".to_string(),
                recipient_id: "@worker:example.test".to_string(),
            },
        ),
        outbox_message_id: Some("outbox-response".to_string()),
    };
    assert!(wake_response_from_outcome(Some(dispatch.clone())).dispatched);
    assert!(!wake_response_from_outcome(None).dispatched);
    assert_eq!(
        project_wake_response_from_dispatch_outcome(Some(dispatch)).outbox_message_id.as_deref(),
        Some("outbox-response")
    );
    assert!(!project_wake_response_from_dispatch_outcome(None).dispatched);
    assert!(!empty_project_wake_response().dispatched);

    let recovery = recovery_response_from_outcome(RecoveryOutcome {
        graced_run_ids: vec![run_id.clone()],
        recovered_run_ids: vec![RunId::new("run-recovered").expect("run id")],
        released_resource_lease_ids: vec![
            crate::agent_scheduler::domain::ResourceLeaseId::new("lease-released")
                .expect("lease id"),
        ],
        tainted_resource_lease_ids: vec![
            crate::agent_scheduler::domain::ResourceLeaseId::new("lease-tainted")
                .expect("lease id"),
        ],
        drifted_worker_ids: vec![worker_id.clone()],
        slept_worker_ids: vec![WorkerName::new("worker-sleep").expect("worker id")],
        outbox_message_ids: vec!["outbox-recovery".to_string()],
    });
    assert_eq!(recovery.recovered_run_ids, vec!["run-recovered"]);
    assert_eq!(recovery.released_resource_lease_ids, vec!["lease-released"]);
    assert_eq!(recovery.slept_worker_ids, vec!["worker-sleep"]);

    let payload = workspace_payload(&AgentSchedulerWorkspaceContextDto {
        slot_id: "slot".to_string(),
        path: "/workspace".to_string(),
        branch: "feature".to_string(),
    });
    assert_eq!(payload["branch"], "feature");
    let worker_payload = worker_event_payload(
        &worker_id,
        "submitted",
        json!({"answer": 42}),
        Some("ok".to_string()),
        None,
        Some(payload.clone()),
    );
    assert_eq!(worker_payload["source"], "matrix_worker");
    assert_eq!(workspace_context_from_value(&payload).expect("workspace").path, "/workspace");
    assert_eq!(
        workspace_context_from_value(&json!({
            "workspace": payload,
        }))
        .expect("nested workspace")
        .slot_id,
        "slot"
    );
    assert_eq!(
        event_error_message(AgentEventKind::RunFailed, None, "message error", &json!({})),
        Some("message error".to_string())
    );
    assert_eq!(
        event_error_message(
            AgentEventKind::HumanIntervened,
            Some(AgentTaskStatus::Failed),
            "status error",
            &json!({})
        ),
        Some("status error".to_string())
    );
    assert_eq!(task_id_from_payload(&json!({"task_id": "task-response"})), Some(task_id));
    assert_eq!(task_id_from_payload(&json!({"task_id": " "})), None);
}

#[test]
fn scheduler_worker_ownership_and_serializer_helpers_cover_error_edges() {
    let task_id = TaskId::new("task-ownership").expect("task id");
    let other_task_id = TaskId::new("task-other").expect("task id");
    let run_id = RunId::new("run-ownership").expect("run id");
    let worker_id = WorkerName::new("worker-owner").expect("worker id");
    let other_worker_id = WorkerName::new("worker-other").expect("worker id");
    let task = StoredAgentTask {
        id: task_id.clone(),
        title: "Worker ownership".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Running,
        spec_uri: "shared/tasks/task-ownership/spec.md".to_string(),
        meta_uri: "shared/tasks/task-ownership/meta.json".to_string(),
        assigned_to: Some(worker_id.clone()),
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({}),
        created_at_ms: TimestampMs(70),
        updated_at_ms: TimestampMs(71),
    };
    let run = StoredAgentRun {
        id: run_id.clone(),
        task_id: task_id.clone(),
        worker_id: Some(worker_id.clone()),
        status: AgentRunStatus::Running,
        attempt: 1,
        lease_acquired_at_ms: None,
        lease_expires_at_ms: None,
        started_at_ms: None,
        finished_at_ms: None,
        last_heartbeat_at_ms: None,
        error_message: None,
    };

    validate_worker_run_ownership(&task, &run, &task_id, &worker_id)
        .expect("matching worker owns run");
    let mut wrong_task_run = run.clone();
    wrong_task_run.task_id = other_task_id;
    assert_eq!(
        validate_worker_run_ownership(&task, &wrong_task_run, &task_id, &worker_id)
            .expect_err("wrong task")
            .to_string(),
        "run run-ownership does not belong to task task-ownership"
    );
    let mut unassigned_run = run.clone();
    unassigned_run.worker_id = None;
    assert_eq!(
        validate_worker_run_ownership(&task, &unassigned_run, &task_id, &worker_id)
            .expect_err("unassigned run")
            .to_string(),
        "run run-ownership is not assigned to a worker"
    );
    let mut wrong_worker_run = run;
    wrong_worker_run.worker_id = Some(other_worker_id);
    assert_eq!(
        validate_worker_run_ownership(&task, &wrong_worker_run, &task_id, &worker_id)
            .expect_err("wrong worker")
            .to_string(),
        "run run-ownership belongs to worker worker-other, got worker-owner"
    );

    assert!(is_default_dispatch_mode(&AgentSchedulerDispatchModeDto::Local));
    assert!(!is_default_dispatch_mode(&AgentSchedulerDispatchModeDto::Auto));
    assert!(is_false(&false));
    assert!(!is_false(&true));
    assert!(task_metadata_object(Value::Null).expect("null metadata").is_empty());
    assert_eq!(
        task_metadata_object(json!(["not", "object"])).expect_err("array metadata").to_string(),
        "task metadata must be a JSON object"
    );
    assert!(reasoning_effort_field(&json!({})).expect("missing effort").is_none());
    assert!(reasoning_effort_field(&json!({"reasoning_effort": null}))
        .expect("null effort")
        .is_none());
    assert_eq!(
        reasoning_effort_field(&json!({"reasoning_effort": "invalid-effort"}))
            .expect_err("invalid effort")
            .to_string(),
        "invalid task metadata field reasoning_effort: unknown variant `invalid-effort`, expected one of `none`, `low`, `medium`, `high`, `xhigh`, `max`"
    );
}

#[test]
fn scheduler_error_helpers_cover_lifecycle_command_and_git_failures() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("error-helper-branches");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    let worker_id = WorkerName::new("worker-error-helper").expect("worker id");
    let task_id = TaskId::new("task-error-helper").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(task_id.as_str(), AgentTaskStatus::Running, &worker_id),
        )
        .expect("seed task");
    let task = gateway.store.load_task(&project_path, &task_id).expect("load task").expect("task");

    assert_eq!(
        gateway
            .resolve_worker_lifecycle_outcome(&project_path, &task, WORKER_OUTCOME_SUCCESS)
            .expect_err("missing workflow")
            .to_string(),
        "task board workflow is required for worker lifecycle outcome"
    );
    let event = append_command_event(
        &gateway.store,
        &project_path,
        &task,
        None,
        AgentSchedulerCommandKindDto::Start,
        AgentTaskStatus::Queued,
        None,
        TimestampMs(120),
    )
    .expect("append command event");
    assert_eq!(event.message, "accepted agent scheduler command start");
    assert!(
        project_current_branch(&project_path)
            .expect_err("non-git project")
            .to_string()
            .contains("merge target branch could not be resolved from project current branch")
    );
    assert_eq!(
        map_scheduler_error(anyhow::anyhow!("scheduler helper error")).to_string(),
        "scheduler helper error"
    );
    assert_eq!(
        map_scheduler_command_error(anyhow::anyhow!(
            "unsupported agent task transition for finite: completed -> queued"
        ))
        .to_string(),
        "unsupported agent task transition for finite: completed -> queued"
    );
}

#[tokio::test]
async fn scheduler_worker_run_events_cover_state_machine_branches() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("worker-event-branches");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    gateway
        .store
        .save_task_board_workflow(
            &project_path,
            &TaskPoolWorkflowSnapshotDto {
                id: "worker_event_flow".to_string(),
                label: "Worker Event Flow".to_string(),
                statuses: vec![
                    workflow_status_with_transitions(
                        "custom_running",
                        TaskPoolWorkflowStatusRoleDto::Executing,
                        vec![
                            workflow_transition("success", "custom_submitted"),
                            workflow_transition("failure", "custom_failed"),
                        ],
                    ),
                    workflow_status_with_transitions(
                        "custom_submitted",
                        TaskPoolWorkflowStatusRoleDto::ReviewReady,
                        vec![workflow_transition("success", "custom_completed")],
                    ),
                    workflow_status_with_transitions(
                        "custom_completed",
                        TaskPoolWorkflowStatusRoleDto::Done,
                        Vec::new(),
                    ),
                    workflow_status_with_transitions(
                        "custom_failed",
                        TaskPoolWorkflowStatusRoleDto::Failed,
                        Vec::new(),
                    ),
                ],
            },
        )
        .expect("save workflow");
    let worker_id = WorkerName::new("worker-events").expect("worker id");

    for (task_name, status) in [
        ("task-ack", AgentTaskStatus::Dispatched),
        ("task-start", AgentTaskStatus::Acknowledged),
        ("task-heartbeat", AgentTaskStatus::Running),
        ("task-submit-idempotent", AgentTaskStatus::Submitted),
        ("task-complete-idempotent", AgentTaskStatus::Completed),
        ("task-fail-idempotent", AgentTaskStatus::Failed),
    ] {
        let initial_status = match status {
            AgentTaskStatus::Submitted | AgentTaskStatus::Completed | AgentTaskStatus::Failed => {
                AgentTaskStatus::Running
            }
            other => other,
        };
        gateway
            .store
            .create_task(&project_path, worker_event_task(task_name, initial_status, &worker_id))
            .expect("seed worker event task");
        claim_matrix_run(
            &gateway.store,
            &project_path,
            task_name,
            &format!("run-{task_name}"),
            worker_id.as_str(),
        );
        if status == AgentTaskStatus::Completed {
            gateway
                .store
                .transition_task_status(
                    &project_path,
                    &TaskId::new(task_name).expect("task id"),
                    AgentTaskStatus::Submitted,
                    TimestampMs(89),
                )
                .expect("transition seeded task")
                .expect("seeded task submitted");
        }
        if status != initial_status {
            gateway
                .store
                .transition_task_status(
                    &project_path,
                    &TaskId::new(task_name).expect("task id"),
                    status,
                    TimestampMs(89),
                )
                .expect("transition seeded task")
                .expect("seeded task transitioned");
        }
    }

    let ack_id = TaskId::new("task-ack").expect("task id");
    let ack_run = RunId::new("run-task-ack").expect("run id");
    let ack = gateway
        .apply_worker_run_event(
            &project_path,
            &ack_id,
            &ack_run,
            &worker_id,
            worker_event_request(
                "worker-events",
                AgentSchedulerWorkerRunEventKindDto::Acknowledged,
                Some("acknowledged explicitly"),
            ),
            TimestampMs(90),
        )
        .expect("ack event");
    assert_eq!(ack.task.status, AgentTaskStatus::Acknowledged);
    assert_eq!(ack.event.expect("ack event").kind, AgentEventKind::WorkerAcknowledged);

    let start_id = TaskId::new("task-start").expect("task id");
    let start_run = RunId::new("run-task-start").expect("run id");
    let started = gateway
        .apply_worker_run_event(
            &project_path,
            &start_id,
            &start_run,
            &worker_id,
            worker_event_request(
                "worker-events",
                AgentSchedulerWorkerRunEventKindDto::Started,
                None,
            ),
            TimestampMs(91),
        )
        .expect("start event");
    assert_eq!(started.task.status, AgentTaskStatus::Running);
    assert_eq!(started.event.expect("start event").kind, AgentEventKind::RunStarted);

    let heartbeat_id = TaskId::new("task-heartbeat").expect("task id");
    let heartbeat_run = RunId::new("run-task-heartbeat").expect("run id");
    let heartbeat = gateway
        .apply_worker_run_event(
            &project_path,
            &heartbeat_id,
            &heartbeat_run,
            &worker_id,
            AgentSchedulerWorkerRunEventRequest {
                lease_expires_at_ms: Some(TimestampMs(200)),
                ..worker_event_request(
                    "worker-events",
                    AgentSchedulerWorkerRunEventKindDto::Heartbeat,
                    None,
                )
            },
            TimestampMs(92),
        )
        .expect("heartbeat event");
    assert!(heartbeat.event.is_none());
    assert_eq!(heartbeat.run.lease_expires_at_ms, Some(TimestampMs(200)));

    for (task_name, event_kind) in [
        ("task-submit-idempotent", AgentSchedulerWorkerRunEventKindDto::Submitted),
        ("task-complete-idempotent", AgentSchedulerWorkerRunEventKindDto::Completed),
        ("task-fail-idempotent", AgentSchedulerWorkerRunEventKindDto::Failed),
    ] {
        let task_id = TaskId::new(task_name).expect("task id");
        let run_id = RunId::new(format!("run-{task_name}")).expect("run id");
        let outcome = gateway
            .apply_worker_run_event(
                &project_path,
                &task_id,
                &run_id,
                &worker_id,
                worker_event_request("worker-events", event_kind, None),
                TimestampMs(93),
            )
            .expect("idempotent worker event");
        assert!(outcome.event.is_none());
    }

    let queued_id = TaskId::new("task-worker-event-queued").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(queued_id.as_str(), AgentTaskStatus::Queued, &worker_id),
        )
        .expect("seed queued worker event task");
    let queued_run = RunId::new("run-worker-event-queued").expect("run id");
    assert_eq!(
        gateway
            .submit_worker_run(
                &project_path,
                &queued_id,
                &queued_run,
                &worker_id,
                None,
                None,
                json!({}),
                None,
                TimestampMs(94),
            )
            .expect_err("submit queued")
            .to_string(),
        "submit requires running task, got queued"
    );
    assert_eq!(
        gateway
            .complete_worker_run(
                &project_path,
                &queued_id,
                &queued_run,
                &worker_id,
                None,
                None,
                json!({}),
                None,
                TimestampMs(94),
            )
            .expect_err("complete queued")
            .to_string(),
        "submit requires running task, got queued"
    );
    let completed_error_id = TaskId::new("task-worker-event-completed-error").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(completed_error_id.as_str(), AgentTaskStatus::Completed, &worker_id),
        )
        .expect("seed completed worker event task");
    assert_eq!(
        gateway
            .fail_worker_run(
                &project_path,
                &completed_error_id,
                &RunId::new("run-worker-event-completed-error").expect("run id"),
                &worker_id,
                None,
                None,
                json!({}),
                None,
                TimestampMs(94),
            )
            .expect_err("fail completed")
            .to_string(),
        "fail requires active task, got completed"
    );

    for (task_name, event_kind, final_status, event_kind_expected) in [
        (
            "task-complete",
            AgentSchedulerWorkerRunEventKindDto::Completed,
            AgentTaskStatus::Completed,
            AgentEventKind::RunCompleted,
        ),
        (
            "task-fail",
            AgentSchedulerWorkerRunEventKindDto::Failed,
            AgentTaskStatus::Failed,
            AgentEventKind::RunFailed,
        ),
    ] {
        gateway
            .store
            .create_task(
                &project_path,
                worker_event_task(task_name, AgentTaskStatus::Running, &worker_id),
            )
            .expect("seed terminal worker event task");
        claim_matrix_run(
            &gateway.store,
            &project_path,
            task_name,
            &format!("run-{task_name}"),
            worker_id.as_str(),
        );
        let task_id = TaskId::new(task_name).expect("task id");
        let run_id = RunId::new(format!("run-{task_name}")).expect("run id");
        let request = AgentSchedulerWorkerRunEventRequest {
            result: Some("worker output".to_string()),
            workspace: Some(AgentSchedulerWorkspaceContextDto {
                slot_id: "slot-worker".to_string(),
                path: project_path.to_string_lossy().to_string(),
                branch: "feature/worker".to_string(),
            }),
            ..worker_event_request("worker-events", event_kind, None)
        };
        let outcome = gateway
            .apply_worker_run_event(
                &project_path,
                &task_id,
                &run_id,
                &worker_id,
                request,
                TimestampMs(95),
            )
            .expect("terminal worker event");
        assert_eq!(outcome.task.status, final_status);
        assert_eq!(outcome.event.expect("terminal event").kind, event_kind_expected);
        assert!(
            gateway
                .store
                .list_task_artifacts(&project_path, &task_id)
                .expect("artifacts")
                .iter()
                .any(|artifact| artifact.kind == "execution_result")
        );
    }
}

#[tokio::test]
async fn scheduler_run_local_task_records_started_run_and_skips_missing_task() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("run-local-direct");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "local direct output".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    let worker_id = WorkerName::new(LOCAL_GATEWAY_WORKER_ID).expect("local worker id");
    let task_id = TaskId::new("task-run-local-direct").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(task_id.as_str(), AgentTaskStatus::Queued, &worker_id),
        )
        .expect("seed local task");

    let started = gateway
        .run_local_task(
            owned_scheduler_context(&project_path).expect("context"),
            task_id.clone(),
            TimestampMs(100),
        )
        .await
        .expect("run local task")
        .expect("local run started");
    assert_eq!(started.task_id, task_id);
    assert_eq!(started.worker_id.as_str(), LOCAL_GATEWAY_WORKER_ID);

    let skipped = gateway
        .run_local_task(
            owned_scheduler_context(&project_path).expect("context"),
            TaskId::new("task-run-local-missing").expect("task id"),
            TimestampMs(101),
        )
        .await
        .expect("missing task should skip");
    assert!(skipped.is_none());
}

#[tokio::test]
async fn scheduler_store_backed_gateway_methods_cover_empty_and_dispatch_edges() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("store-backed-methods");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "store backed output".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    let context = owned_scheduler_context(&project_path).expect("context");

    let created = gateway
        .create_task_scoped(context.clone(), create_task_request("task-store-backed-scoped"))
        .await
        .expect("scoped create");
    assert_eq!(created.task_id, "task-store-backed-scoped");
    let duplicate = gateway
        .create_task_scoped(context.clone(), create_task_request("task-store-backed-scoped"))
        .await
        .expect("duplicate scoped create");
    assert_eq!(duplicate.task_id, "task-store-backed-scoped");
    let events = gateway.store.list_events_after(&project_path, None, 20).expect("stored events");
    assert_eq!(events.iter().filter(|event| event.kind == AgentEventKind::TaskCreated).count(), 1);

    let listed = gateway
        .list_tasks(AgentSchedulerTasksQuery {
            project_path: project_path.to_string_lossy().to_string(),
        })
        .await
        .expect("list tasks");
    assert_eq!(listed.tasks.len(), 1);
    let listed_events = gateway
        .list_events(AgentSchedulerEventsQuery {
            project_path: project_path.to_string_lossy().to_string(),
            after_cursor: None,
            limit: Some(1000),
        })
        .await
        .expect("list events");
    assert_eq!(listed_events.cursor, events.last().expect("event").cursor);
    let recovery = gateway
        .recover(AgentSchedulerRecoveryRequest {
            project_path: project_path.to_string_lossy().to_string(),
            now_ms: TimestampMs(110),
        })
        .await
        .expect("recover empty project");
    assert!(recovery.recovered_run_ids.is_empty());
    let drained = gateway
        .drain_outbox(AgentSchedulerOutboxDrainRequest {
            project_path: project_path.to_string_lossy().to_string(),
            now_ms: TimestampMs(111),
            limit: Some(1000),
        })
        .await
        .expect("drain empty outbox");
    assert_eq!(drained.claimed, 0);

    let local_started = gateway
        .wake_project(
            project_id.clone(),
            context.clone(),
            AgentSchedulerProjectWakeRequest {
                now_ms: TimestampMs(112),
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                auto_promote_pool_tasks: false,
            },
        )
        .await
        .expect("local wake starts queued task");
    assert!(local_started.dispatched);
    assert_eq!(local_started.task_id.as_deref(), Some("task-store-backed-scoped"));
    let matrix_empty = gateway
        .wake_project(
            project_id.clone(),
            context.clone(),
            AgentSchedulerProjectWakeRequest {
                now_ms: TimestampMs(113),
                dispatch_mode: AgentSchedulerDispatchModeDto::Matrix,
                max_concurrent: None,
                auto_promote_pool_tasks: false,
            },
        )
        .await
        .expect("matrix wake empty");
    assert!(!matrix_empty.dispatched);

    let draft_id = TaskId::new("task-ensure-draft").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(
                draft_id.as_str(),
                AgentTaskStatus::Draft,
                &WorkerName::new(LOCAL_GATEWAY_WORKER_ID).expect("worker id"),
            ),
        )
        .expect("seed draft task");
    let draft =
        gateway.store.load_task(&project_path, &draft_id).expect("load draft").expect("draft task");
    gateway
        .ensure_queued_for_local_run(&project_path, &draft, TimestampMs(114))
        .expect("draft promotes to queued");
    let after_draft = gateway
        .store
        .load_task(&project_path, &draft_id)
        .expect("load after draft")
        .expect("draft after");
    assert_eq!(after_draft.status, AgentTaskStatus::Queued);
    let completed_id = TaskId::new("task-ensure-completed").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(
                completed_id.as_str(),
                AgentTaskStatus::Completed,
                &WorkerName::new(LOCAL_GATEWAY_WORKER_ID).expect("worker id"),
            ),
        )
        .expect("seed completed task");
    let completed = gateway
        .store
        .load_task(&project_path, &completed_id)
        .expect("load completed")
        .expect("completed task");
    assert_eq!(
        gateway
            .ensure_queued_for_local_run(&project_path, &completed, TimestampMs(115))
            .expect_err("completed cannot queue")
            .to_string(),
        "command requires queued task, got completed"
    );

    let matrix_task_id = TaskId::new("task-matrix-no-worker").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(
                matrix_task_id.as_str(),
                AgentTaskStatus::Queued,
                &WorkerName::new("matrix-missing").expect("worker id"),
            ),
        )
        .expect("seed matrix task");
    let matrix_dispatch = gateway
        .run_or_dispatch_project_task(
            &project_path,
            &matrix_task_id,
            TimestampMs(116),
            AgentSchedulerDispatchModeDto::Matrix,
            Some(10),
        )
        .await
        .expect("matrix dispatch without worker");
    assert!(matrix_dispatch.is_none());
}

#[tokio::test]
async fn scheduler_store_snapshot_controller_reflects_worker_lifecycle_requests() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("controller-project");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .upsert_worker_snapshot(
            &project_path,
            NewAgentWorkerSnapshot {
                id: WorkerName::new("controller-worker").expect("worker id"),
                runtime: Some("matrix".to_string()),
                deployment: Some("local".to_string()),
                model: "auto".to_string(),
                model_provider: None,
                status: AgentWorkerStatus::Idle,
                desired_state: Some("Sleeping".to_string()),
                team_id: Some(TeamId::new("controller-team").expect("team id")),
                worker_role: Some("reviewer".to_string()),
                matrix_user_id: Some("@worker:example.test".to_string()),
                room_id: Some("!room:example.test".to_string()),
                container_state: Some("paused".to_string()),
                idle_since_ms: None,
                last_seen_at_ms: None,
                last_heartbeat_at: None,
                capabilities: Vec::new(),
                updated_at_ms: TimestampMs(34),
            },
        )
        .expect("seed worker");
    let controller = StoreSnapshotController { store: &store, project_path: &project_path };
    let worker_name = WorkerName::new("controller-worker").expect("worker id");
    let ready = controller
        .ensure_worker_ready(EnsureWorkerReadyRequest {
            worker_name: worker_name.clone(),
            runtime: Some("local".to_string()),
            team_id: None,
            worker_role: None,
        })
        .await
        .expect("ready snapshot");
    assert_eq!(ready.desired_state, WorkerLifecycleState::Running);
    assert_eq!(ready.runtime.as_deref(), Some("matrix"));
    assert_eq!(ready.room_id.as_deref(), Some("!room:example.test"));
    assert_eq!(ready.container_state.as_deref(), Some("paused"));

    let sleeping = controller
        .request_worker_sleep(WorkerLifecycleRequest {
            worker_name: worker_name.clone(),
            desired_state: WorkerLifecycleState::Sleeping,
            runtime: None,
            team_id: None,
            worker_role: None,
        })
        .await
        .expect("sleep snapshot");
    assert_eq!(sleeping.desired_state, WorkerLifecycleState::Sleeping);

    let stopped = controller
        .request_worker_stop(WorkerLifecycleRequest {
            worker_name: worker_name.clone(),
            desired_state: WorkerLifecycleState::Stopped,
            runtime: None,
            team_id: None,
            worker_role: None,
        })
        .await
        .expect("stop snapshot");
    assert_eq!(stopped.desired_state, WorkerLifecycleState::Stopped);

    let status = controller
        .worker_status(WorkerStatusRequest { worker_name: worker_name.clone() })
        .await
        .expect("worker status")
        .expect("worker snapshot");
    assert_eq!(status.desired_state, WorkerLifecycleState::Sleeping);
    let missing = controller
        .worker_status(WorkerStatusRequest {
            worker_name: WorkerName::new("missing-worker").expect("worker id"),
        })
        .await
        .expect("missing worker");
    assert!(missing.is_none());
    assert_eq!(
        worker_snapshot(
            WorkerName::new("standalone").expect("worker id"),
            WorkerLifecycleState::Running,
            Some("runtime".to_string()),
            None,
            Some("role".to_string()),
            None,
            None,
        )
        .worker_role
        .as_deref(),
        Some("role")
    );
}

#[tokio::test]
async fn scheduler_project_command_branches_transition_or_reject_tasks() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("command-branches");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    for (task_id, status) in [
        ("task-pause", AgentTaskStatus::Running),
        ("task-cancel", AgentTaskStatus::Queued),
        ("task-archive", AgentTaskStatus::Completed),
        ("task-retry", AgentTaskStatus::Failed),
        ("task-retry-invalid", AgentTaskStatus::Queued),
        ("task-unsupported", AgentTaskStatus::Queued),
    ] {
        gateway
            .store
            .create_task(
                &project_path,
                NewAgentTask {
                    id: TaskId::new(task_id).expect("task id"),
                    title: format!("Command branch {task_id}"),
                    kind: AgentTaskKind::Finite,
                    status,
                    spec_uri: format!("shared/tasks/{task_id}/spec.md"),
                    meta_uri: format!("shared/tasks/{task_id}/meta.json"),
                    assigned_to: None,
                    team_id: None,
                    room_id: None,
                    project_room_id: None,
                    delegated_to_team: None,
                    schedule: None,
                    timezone: None,
                    next_scheduled_at: None,
                    last_executed_at: None,
                    dag: Vec::new(),
                    metadata: json!({"prompt": task_id}),
                    created_at_ms: TimestampMs(50),
                },
            )
            .expect("seed task");
    }

    for (task_id, command, expected_status) in [
        ("task-pause", AgentSchedulerCommandKindDto::Pause, AgentTaskStatus::Paused),
        ("task-cancel", AgentSchedulerCommandKindDto::Cancel, AgentTaskStatus::Cancelled),
        ("task-archive", AgentSchedulerCommandKindDto::Archive, AgentTaskStatus::Archived),
    ] {
        let task_id = TaskId::new(task_id).expect("task id");
        let task =
            gateway.store.load_task(&project_path, &task_id).expect("load task").expect("task");
        gateway
            .apply_project_command(
                &project_id,
                &project_path,
                &task,
                command,
                AgentSchedulerDispatchModeDto::Local,
                None,
                TimestampMs(51),
            )
            .await
            .expect("command branch");
        let after = gateway
            .store
            .load_task(&project_path, &task_id)
            .expect("load after")
            .expect("task after");
        assert_eq!(after.status, expected_status);
    }

    let retry_invalid_id = TaskId::new("task-retry-invalid").expect("task id");
    let retry_invalid = gateway
        .store
        .load_task(&project_path, &retry_invalid_id)
        .expect("load retry invalid")
        .expect("retry invalid");
    assert_eq!(
        gateway
            .apply_project_command(
                &project_id,
                &project_path,
                &retry_invalid,
                AgentSchedulerCommandKindDto::Retry,
                AgentSchedulerDispatchModeDto::Matrix,
                None,
                TimestampMs(52),
            )
            .await
            .expect_err("retry requires failed")
            .to_string(),
        "retry requires a failed task"
    );

    let retry_id = TaskId::new("task-retry").expect("task id");
    let retry =
        gateway.store.load_task(&project_path, &retry_id).expect("load retry").expect("retry task");
    let retry_outcome = gateway
        .apply_project_command(
            &project_id,
            &project_path,
            &retry,
            AgentSchedulerCommandKindDto::Retry,
            AgentSchedulerDispatchModeDto::Matrix,
            None,
            TimestampMs(53),
        )
        .await
        .expect("retry command");
    assert!(retry_outcome.is_none());
    let after_retry = gateway
        .store
        .load_task(&project_path, &retry_id)
        .expect("load after retry")
        .expect("retry after");
    assert_eq!(after_retry.status, AgentTaskStatus::Queued);

    let unsupported_id = TaskId::new("task-unsupported").expect("task id");
    let unsupported = gateway
        .store
        .load_task(&project_path, &unsupported_id)
        .expect("load unsupported")
        .expect("unsupported task");
    assert_eq!(
        gateway
            .apply_project_command(
                &project_id,
                &project_path,
                &unsupported,
                AgentSchedulerCommandKindDto::RunStageAction,
                AgentSchedulerDispatchModeDto::Local,
                None,
                TimestampMs(54),
            )
            .await
            .expect_err("unsupported command")
            .to_string(),
        "unsupported agent scheduler command: run_stage_action"
    );
}

#[tokio::test]
async fn scheduler_task_board_command_payload_errors_are_reported_before_store_mutation() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("task-board-command-errors");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    let task_id = TaskId::new("task-board-errors").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            NewAgentTask {
                id: task_id.clone(),
                title: "Task board command errors".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-board-errors/spec.md".to_string(),
                meta_uri: "shared/tasks/task-board-errors/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({"prompt": "errors"}),
                created_at_ms: TimestampMs(60),
            },
        )
        .expect("seed task");

    for (command, expected) in [
        (AgentSchedulerCommandKindDto::Move, "move command requires move_task payload"),
        (
            AgentSchedulerCommandKindDto::UpdateMetadata,
            "update_metadata command requires metadata_update payload",
        ),
        (
            AgentSchedulerCommandKindDto::ImportLegacyTask,
            "import_legacy_task command requires legacy_import payload",
        ),
        (
            AgentSchedulerCommandKindDto::AssignStaff,
            "assign_staff command requires staff_command payload",
        ),
        (
            AgentSchedulerCommandKindDto::RunStageAction,
            "run_stage_action command requires stage_action payload",
        ),
        (AgentSchedulerCommandKindDto::Start, "unsupported task board command dispatch: start"),
    ] {
        let error = gateway
            .apply_task_board_command(
                &project_id,
                &project_path,
                &task_id,
                AgentSchedulerCommandRequest {
                    task_id: "task-board-errors".into(),
                    command,
                    dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                    max_concurrent: None,
                    reason: None,
                    payload: AgentSchedulerCommandPayloadDto::default(),
                },
                TimestampMs(61),
            )
            .await
            .expect_err("missing payload should fail");
        assert_eq!(error.to_string(), expected);
    }
}

#[tokio::test]
async fn scheduler_task_board_command_success_branches_return_task_and_event_payloads() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("task-board-command-success");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");
    let task_id = TaskId::new("task-board-success").expect("task id");
    gateway
        .store
        .create_task(
            &project_path,
            worker_event_task(
                task_id.as_str(),
                AgentTaskStatus::Queued,
                &WorkerName::new("worker-board-success").expect("worker id"),
            ),
        )
        .expect("seed task board task");

    let updated = gateway
        .apply_task_board_command(
            &project_id,
            &project_path,
            &task_id,
            AgentSchedulerCommandRequest {
                task_id: task_id.as_str().into(),
                command: AgentSchedulerCommandKindDto::UpdateMetadata,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason: Some("metadata update".to_string()),
                payload: AgentSchedulerCommandPayloadDto {
                    metadata_update: Some(AgentSchedulerTaskMetadataPatchDto {
                        title: Some("Updated title".to_string()),
                        status_key: Some("updated_status".to_string()),
                        ..AgentSchedulerTaskMetadataPatchDto::default()
                    }),
                    ..AgentSchedulerCommandPayloadDto::default()
                },
            },
            TimestampMs(63),
        )
        .await
        .expect("metadata update command");
    assert!(updated.accepted);
    assert_eq!(
        updated.task.expect("updated task").read_model.title.as_deref(),
        Some("Updated title")
    );
    assert!(updated.event.is_some());

    let deleted = gateway
        .apply_task_board_command(
            &project_id,
            &project_path,
            &task_id,
            AgentSchedulerCommandRequest {
                task_id: task_id.as_str().into(),
                command: AgentSchedulerCommandKindDto::SoftDelete,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason: Some("soft delete".to_string()),
                payload: AgentSchedulerCommandPayloadDto::default(),
            },
            TimestampMs(64),
        )
        .await
        .expect("soft delete command");
    assert!(deleted.accepted);
    let restored = gateway
        .apply_task_board_command(
            &project_id,
            &project_path,
            &task_id,
            AgentSchedulerCommandRequest {
                task_id: task_id.as_str().into(),
                command: AgentSchedulerCommandKindDto::Restore,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason: Some("restore".to_string()),
                payload: AgentSchedulerCommandPayloadDto::default(),
            },
            TimestampMs(65),
        )
        .await
        .expect("restore command");
    assert!(restored.accepted);

    let imported = gateway
        .apply_task_board_command(
            &project_id,
            &project_path,
            &task_id,
            AgentSchedulerCommandRequest {
                task_id: task_id.as_str().into(),
                command: AgentSchedulerCommandKindDto::ImportLegacyTask,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason: Some("import legacy".to_string()),
                payload: AgentSchedulerCommandPayloadDto {
                    legacy_import: Some(AgentSchedulerLegacyTaskImportDto {
                        task_id: "legacy-board-success".into(),
                        title: "Legacy board success".to_string(),
                        status_key: Some("legacy_status".to_string()),
                        status: Some(AgentTaskStatusDto::Queued),
                        priority: 2,
                        order_no: 4,
                        model: "auto".to_string(),
                        agent: Some("agent".to_string()),
                        acp_agent: None,
                        reasoning_effort: None,
                        prompt: "legacy prompt".to_string(),
                        workflow_id: None,
                        metadata: json!({"source": "legacy"}),
                        created_at_ms: TimestampMs(66),
                        updated_at_ms: Some(TimestampMs(67)),
                    }),
                    ..AgentSchedulerCommandPayloadDto::default()
                },
            },
            TimestampMs(66),
        )
        .await
        .expect("legacy import command");
    assert!(imported.accepted);
    assert_eq!(imported.task.expect("imported task").task_id.as_ref(), "legacy-board-success");
}

#[tokio::test]
async fn scheduler_merge_early_rejections_append_diagnostics_without_git_merge() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("merge-early-rejections");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unused".to_string(),
    });
    gateway.store.initialize_project(&project_path).expect("init scheduler");

    let cases = [
        (
            "merge-already-completed",
            AgentTaskStatus::Completed,
            json!({
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "feature/completed",
                "merge_source_branch": "feature/completed",
                "merge_target_branch": "main",
            }),
            None,
            "agent scheduler merge skipped: task already completed",
        ),
        (
            "merge-wrong-status",
            AgentTaskStatus::Queued,
            json!({
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "feature/wrong-status",
                "merge_source_branch": "feature/wrong-status",
                "merge_target_branch": "main",
            }),
            Some("merge requires submitted task, got queued"),
            "agent scheduler merge rejected: status=queued",
        ),
        (
            "merge-missing-path",
            AgentTaskStatus::Submitted,
            json!({
                "selected_worktree_branch": "feature/missing-path",
                "merge_source_branch": "feature/missing-path",
                "merge_target_branch": "main",
            }),
            Some("selected_worktree_path is required"),
            "agent scheduler merge rejected: selected_worktree_path missing",
        ),
        (
            "merge-missing-branch",
            AgentTaskStatus::Submitted,
            json!({
                "selected_worktree_path": project_path.to_string_lossy(),
                "merge_target_branch": "main",
            }),
            Some("selected_worktree_branch is required"),
            "agent scheduler merge rejected: selected_worktree_branch missing",
        ),
        (
            "merge-unresolved-source",
            AgentTaskStatus::Submitted,
            json!({
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "HEAD",
                "merge_target_branch": "main",
            }),
            Some("merge_source_branch could not be resolved from selected_worktree_branch"),
            "agent scheduler merge rejected: merge_source_branch unresolved",
        ),
        (
            "merge-same-branch",
            AgentTaskStatus::Submitted,
            json!({
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "main",
                "merge_source_branch": "main",
                "merge_target_branch": "main",
            }),
            Some("invalid merge branches: source=main target=main"),
            "agent scheduler merge rejected: invalid branches",
        ),
        (
            "merge-selected-source-mismatch",
            AgentTaskStatus::Submitted,
            json!({
                "selected_worktree_path": project_path.to_string_lossy(),
                "selected_worktree_branch": "feature/selected",
                "merge_source_branch": "feature/source",
                "merge_target_branch": "main",
            }),
            Some(
                "selected_worktree_branch must match merge_source_branch: feature/selected != feature/source",
            ),
            "agent scheduler merge rejected: selected_branch=feature/selected source_branch=feature/source",
        ),
        (
            "merge-missing-worktree-dir",
            AgentTaskStatus::Submitted,
            json!({
                "selected_worktree_path": project_path.join("missing").to_string_lossy(),
                "selected_worktree_branch": "feature/missing-dir",
                "merge_source_branch": "feature/missing-dir",
                "merge_target_branch": "main",
            }),
            Some("selected_worktree_path must reference an existing directory"),
            "agent scheduler merge rejected: selected_worktree_path missing path=",
        ),
    ];

    for (task_id, status, metadata, expected_error, expected_log) in cases {
        let mut task = submitted_merge_task(task_id, &project_path, "feature/default", true);
        task.status = status;
        task.metadata = metadata;
        gateway.store.create_task(&project_path, task).expect("seed merge task");
        let task_id = TaskId::new(task_id).expect("task id");
        let task =
            gateway.store.load_task(&project_path, &task_id).expect("load task").expect("task");

        let result =
            gateway.merge_project_task(&project_id, &project_path, &task, TimestampMs(62)).await;
        match expected_error {
            Some(expected) => {
                assert_eq!(result.expect_err("merge should reject").to_string(), expected);
            }
            None => result.expect("completed task should be idempotent"),
        }
        let logs = gateway.store.list_task_logs(&project_path, &task_id).expect("logs");
        assert!(
            logs.iter().any(|log| log.message.contains(expected_log)),
            "missing diagnostic {expected_log}; logs: {logs:?}"
        );
    }
}

#[derive(Default)]
struct MinimalGateway;

#[async_trait::async_trait]
impl AgentSchedulerGateway for MinimalGateway {
    async fn create_task(
        &self,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        let task = AgentTaskDto {
            id: request.task_id,
            title: request.title,
            kind: request.kind,
            status: request.status.unwrap_or(AgentTaskStatusDto::Queued),
            assigned_to: request.assigned_to,
            team_id: request.team_id,
            room_id: request.room_id,
            project_room_id: request.project_room_id,
            delegated_to_team: request.delegated_to_team,
            schedule: request.schedule,
            timezone: request.timezone,
            next_scheduled_at: request.next_scheduled_at,
            last_executed_at: request.last_executed_at,
            dag: Vec::new(),
            created_at_ms: request.created_at_ms,
            updated_at_ms: request.created_at_ms,
            metadata: request.metadata,
        };
        Ok(AgentSchedulerCreateTaskResponse { task_id: task.id.clone(), task })
    }

    async fn list_tasks(
        &self,
        _query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError> {
        Ok(AgentSchedulerTasksResponse { tasks: Vec::new() })
    }

    async fn wake(
        &self,
        _request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        Ok(AgentSchedulerWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        })
    }

    async fn recover(
        &self,
        _request: AgentSchedulerRecoveryRequest,
    ) -> Result<AgentSchedulerRecoveryResponse, ApiError> {
        Ok(AgentSchedulerRecoveryResponse {
            graced_run_ids: Vec::new(),
            recovered_run_ids: Vec::new(),
            released_resource_lease_ids: Vec::new(),
            tainted_resource_lease_ids: Vec::new(),
            drifted_worker_ids: Vec::new(),
            slept_worker_ids: Vec::new(),
            outbox_message_ids: Vec::new(),
        })
    }

    async fn list_events(
        &self,
        _query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError> {
        Ok(AgentSchedulerEventsResponse { cursor: 0, events: Vec::new() })
    }

    async fn drain_outbox(
        &self,
        _request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError> {
        Ok(AgentSchedulerOutboxDrainResponse { claimed: 0, messages: Vec::new() })
    }
}

fn default_context() -> AgentSchedulerContext {
    let tenant = TenantScope::new("default-platform", "default-company").expect("tenant");
    let scope = ProjectScope::new(tenant, ProjectId::from("default-project")).expect("scope");
    AgentSchedulerContext::new(AgentSchedulerScope::new(scope), "/tmp/default-project")
        .expect("context")
}

fn create_task_request(task_id: &str) -> AgentSchedulerCreateTaskRequest {
    AgentSchedulerCreateTaskRequest {
        task_id: task_id.to_string(),
        title: "Create through gateway".to_string(),
        kind: AgentTaskKindDto::Finite,
        status: Some(AgentTaskStatusDto::Queued),
        spec_uri: "spec.md".to_string(),
        meta_uri: "meta.json".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        metadata: json!({}),
        created_at_ms: TimestampMs(40),
    }
}

fn worker_event_task(
    task_id: &str,
    status: AgentTaskStatus,
    worker_id: &WorkerName,
) -> NewAgentTask {
    NewAgentTask {
        id: TaskId::new(task_id).expect("task id"),
        title: format!("Worker event {task_id}"),
        kind: AgentTaskKind::Finite,
        status,
        spec_uri: format!("shared/tasks/{task_id}/spec.md"),
        meta_uri: format!("shared/tasks/{task_id}/meta.json"),
        assigned_to: Some(worker_id.clone()),
        team_id: None,
        room_id: Some("!worker-event-room:localhost".to_string()),
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "task_board.workflow_id": "worker_event_flow",
            "task_board.status_key": "custom_running",
            "task_board.status_role": "executing",
            "prompt": task_id,
        }),
        created_at_ms: TimestampMs(89),
    }
}

fn worker_event_request(
    worker_id: &str,
    event: AgentSchedulerWorkerRunEventKindDto,
    message: Option<&str>,
) -> AgentSchedulerWorkerRunEventRequest {
    AgentSchedulerWorkerRunEventRequest {
        platform_id: vw_api_types::id::DEFAULT_PLATFORM_ID.to_string(),
        company_id: vw_api_types::id::DEFAULT_COMPANY_ID.to_string(),
        project_id: "worker-event-project".to_string(),
        worker_id: worker_id.to_string(),
        event,
        message: message.map(str::to_string),
        result: None,
        error_message: None,
        workspace: None,
        payload: json!({"source": "unit"}),
        now_ms: None,
        lease_expires_at_ms: None,
    }
}

#[tokio::test]
async fn scheduler_default_gateway_methods_report_unconfigured_project_routes() {
    let gateway = MinimalGateway;
    let context = default_context();
    let created = gateway
        .create_task_scoped(context.clone(), create_task_request("task-default-scope"))
        .await
        .expect("default scoped create delegates");
    assert_eq!(created.task_id, "task-default-scope");

    let command = AgentSchedulerCommandRequest {
        task_id: "task-default".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
        max_concurrent: None,
        reason: None,
        payload: AgentSchedulerCommandPayloadDto::default(),
    };
    let bulk =
        AgentSchedulerBulkCommandRequest { commands: vec![command.clone()], stop_on_error: false };
    let metadata = AgentSchedulerTaskMetadataUpdateRequest {
        patch: AgentSchedulerTaskMetadataPatchDto::default(),
        reason: None,
    };
    let legacy =
        AgentSchedulerLegacyImportRequest { workflow: None, tasks: Vec::new(), dry_run: false };
    let worker = AgentSchedulerUpsertWorkerRequest {
        worker_id: "worker-default".to_string(),
        runtime: None,
        deployment: None,
        model: None,
        model_provider: None,
        status: None,
        desired_state: None,
        team_id: None,
        worker_role: None,
        matrix_user_id: None,
        room_id: None,
        container_state: None,
        idle_since_ms: None,
        last_seen_at_ms: None,
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: None,
    };
    let team = AgentSchedulerUpsertTeamRequest {
        team_id: "team-default".to_string(),
        name: "Default team".to_string(),
        status: None,
        leader_id: "worker-default".to_string(),
        team_room_id: None,
        leader_dm_room_id: None,
        leader_ready: false,
        ready_workers: 0,
        total_workers: 0,
        members: Vec::new(),
        updated_at_ms: None,
    };
    let worker_event = AgentSchedulerWorkerRunEventRequest {
        platform_id: "default-platform".to_string(),
        company_id: "default-company".to_string(),
        project_id: "default-project".to_string(),
        worker_id: "worker-default".to_string(),
        event: AgentSchedulerWorkerRunEventKindDto::Submitted,
        message: None,
        result: None,
        error_message: None,
        workspace: None,
        payload: json!({}),
        now_ms: None,
        lease_expires_at_ms: None,
    };
    let wake = AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(41),
        dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
        max_concurrent: None,
        auto_promote_pool_tasks: false,
    };
    let outbox = AgentSchedulerProjectOutboxDrainRequest { now_ms: TimestampMs(42), limit: None };

    let checks = [
        gateway.command("project".to_string(), context.clone(), command).await,
        gateway.bulk_command("project".to_string(), context.clone(), bulk).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway.task_board_workflow("project".to_string(), context.clone()).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway.task_board_status("project".to_string(), context.clone()).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway
            .update_task_metadata(
                "project".to_string(),
                context.clone(),
                "task".to_string(),
                metadata,
            )
            .await
            .map(|_| AgentSchedulerProjectCommandResponse {
                accepted: false,
                task: None,
                event: None,
            }),
        gateway.import_legacy_tasks("project".to_string(), context.clone(), legacy).await.map(
            |_| AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None },
        ),
        gateway.list_workers(context.clone()).await.map(|_| AgentSchedulerProjectCommandResponse {
            accepted: false,
            task: None,
            event: None,
        }),
        gateway.upsert_worker(context.clone(), worker).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway.list_teams(context.clone()).await.map(|_| AgentSchedulerProjectCommandResponse {
            accepted: false,
            task: None,
            event: None,
        }),
        gateway.upsert_team(context.clone(), team).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway.task_logs(context.clone(), "task".to_string()).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway.task_artifacts(context.clone(), "task".to_string()).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway
            .worker_run_event(
                "project".to_string(),
                context.clone(),
                "task".to_string(),
                "run".to_string(),
                worker_event,
            )
            .await
            .map(|_| AgentSchedulerProjectCommandResponse {
                accepted: false,
                task: None,
                event: None,
            }),
        gateway
            .task_board_events(
                "project".to_string(),
                context.clone(),
                AgentSchedulerProjectEventsQuery { after_cursor: None, limit: None },
            )
            .await
            .map(|_| AgentSchedulerProjectCommandResponse {
                accepted: false,
                task: None,
                event: None,
            }),
        gateway.wake_project("project".to_string(), context.clone(), wake).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
        gateway.drain_project_outbox(context, outbox).await.map(|_| {
            AgentSchedulerProjectCommandResponse { accepted: false, task: None, event: None }
        }),
    ];
    for result in checks {
        assert!(result.expect_err("default method should fail").to_string().contains("gateway"));
    }
}

#[tokio::test]
async fn scheduler_unscoped_handlers_reject_invalid_or_missing_context() {
    assert_eq!(
        agent_scheduler_create_task(Json(create_task_request("task-unscoped")))
            .await
            .expect_err("unscoped create should fail")
            .to_string(),
        "explicit agent scheduler scope is required"
    );
    assert_eq!(
        agent_scheduler_list_tasks(Query(AgentSchedulerTasksQuery {
            project_path: " ".to_string()
        }))
        .await
        .expect_err("empty project path")
        .to_string(),
        "project_path must not be empty"
    );
    assert_eq!(
        agent_scheduler_wake(Json(AgentSchedulerWakeRequest {
            project_path: " ".to_string(),
            now_ms: TimestampMs(43),
            dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
            max_concurrent: None,
            auto_promote_pool_tasks: false,
        }))
        .await
        .expect_err("empty wake path")
        .to_string(),
        "project_path must not be empty"
    );
    assert_eq!(
        agent_scheduler_recovery(Json(AgentSchedulerRecoveryRequest {
            project_path: " ".to_string(),
            now_ms: TimestampMs(44),
        }))
        .await
        .expect_err("empty recovery path")
        .to_string(),
        "project_path must not be empty"
    );
    assert_eq!(
        agent_scheduler_list_events(Query(AgentSchedulerEventsQuery {
            project_path: " ".to_string(),
            after_cursor: None,
            limit: None,
        }))
        .await
        .expect_err("empty events path")
        .to_string(),
        "project_path must not be empty"
    );
    assert_eq!(
        agent_scheduler_drain_outbox(Json(AgentSchedulerOutboxDrainRequest {
            project_path: " ".to_string(),
            now_ms: TimestampMs(45),
            limit: None,
        }))
        .await
        .expect_err("empty outbox path")
        .to_string(),
        "project_path must not be empty"
    );
}

#[tokio::test]
async fn scheduler_project_route_wrappers_resolve_context_before_gateway_calls() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-route-wrappers");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let tenant = project::default_tenant_scope();
    let gateway: SharedAgentSchedulerGateway = Arc::new(MinimalGateway);

    let created = agent_scheduler_project_create_task(
        AxumPath(project_id.clone()),
        Extension(tenant.clone()),
        Extension(Arc::clone(&gateway)),
        Json(create_task_request("task-project-wrapper")),
    )
    .await
    .expect("project scoped create")
    .0;
    assert_eq!(created.task_id, "task-project-wrapper");

    assert_eq!(
        agent_scheduler_project_drain_outbox(
            AxumPath(project_id),
            Extension(tenant),
            Extension(gateway),
            Json(AgentSchedulerProjectOutboxDrainRequest {
                now_ms: TimestampMs(70),
                limit: Some(1),
            }),
        )
        .await
        .expect_err("default drain should fail")
        .to_string(),
        "agent scheduler project outbox gateway is not configured"
    );
}

#[tokio::test]
async fn scheduler_project_route_wrappers_cover_gateway_defaults_and_scope_validation() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-route-defaults");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let tenant = project::default_tenant_scope();
    let gateway: SharedAgentSchedulerGateway = Arc::new(MinimalGateway);

    let command = AgentSchedulerCommandRequest {
        task_id: "task-route-default".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
        max_concurrent: None,
        reason: None,
        payload: AgentSchedulerCommandPayloadDto::default(),
    };
    let metadata = AgentSchedulerTaskMetadataUpdateRequest {
        patch: AgentSchedulerTaskMetadataPatchDto::default(),
        reason: None,
    };
    let legacy =
        AgentSchedulerLegacyImportRequest { workflow: None, tasks: Vec::new(), dry_run: true };
    let worker = AgentSchedulerUpsertWorkerRequest {
        worker_id: "worker-route".to_string(),
        runtime: None,
        deployment: None,
        model: None,
        model_provider: None,
        status: None,
        desired_state: None,
        team_id: None,
        worker_role: None,
        matrix_user_id: None,
        room_id: None,
        container_state: None,
        idle_since_ms: None,
        last_seen_at_ms: None,
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: None,
    };
    let team = AgentSchedulerUpsertTeamRequest {
        team_id: "team-route".to_string(),
        name: "Route team".to_string(),
        status: None,
        leader_id: "worker-route".to_string(),
        team_room_id: None,
        leader_dm_room_id: None,
        leader_ready: false,
        ready_workers: 0,
        total_workers: 0,
        members: Vec::new(),
        updated_at_ms: None,
    };
    let worker_event = AgentSchedulerWorkerRunEventRequest {
        platform_id: tenant.platform_id().as_ref().to_string(),
        company_id: tenant.company_id().as_ref().to_string(),
        project_id: project_id.clone(),
        worker_id: "worker-route".to_string(),
        event: AgentSchedulerWorkerRunEventKindDto::Heartbeat,
        message: None,
        result: None,
        error_message: None,
        workspace: None,
        payload: json!({}),
        now_ms: None,
        lease_expires_at_ms: None,
    };

    let checks = [
        agent_scheduler_project_command(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(command.clone()),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_bulk_command(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(AgentSchedulerBulkCommandRequest { commands: vec![command], stop_on_error: true }),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_workflow(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_tasks(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_update_task_metadata(
            AxumPath((project_id.clone(), "task-route-default".to_string())),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(metadata),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_legacy_import(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(legacy),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_workers(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_upsert_worker(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(worker),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_teams(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_upsert_team(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(team),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_logs(
            AxumPath((project_id.clone(), "task-route-default".to_string())),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_artifacts(
            AxumPath((project_id.clone(), "task-route-default".to_string())),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_worker_run_event(
            AxumPath((
                project_id.clone(),
                "task-route-default".to_string(),
                "run-route-default".to_string(),
            )),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(worker_event.clone()),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_events(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Query(AgentSchedulerProjectEventsQuery { after_cursor: None, limit: None }),
        )
        .await
        .map(|_| ()),
        agent_scheduler_project_wake(
            AxumPath(project_id.clone()),
            Extension(tenant.clone()),
            Extension(Arc::clone(&gateway)),
            Json(AgentSchedulerProjectWakeRequest {
                now_ms: TimestampMs(80),
                dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
                max_concurrent: None,
                auto_promote_pool_tasks: false,
            }),
        )
        .await
        .map(|_| ()),
    ];
    for result in checks {
        assert!(result.expect_err("default route should fail").to_string().contains("gateway"));
    }

    let mut mismatched_event = worker_event;
    mismatched_event.project_id = "different-project".to_string();
    assert_eq!(
        agent_scheduler_project_worker_run_event(
            AxumPath((
                project_id,
                "task-route-default".to_string(),
                "run-route-default".to_string(),
            )),
            Extension(tenant),
            Extension(gateway),
            Json(mismatched_event),
        )
        .await
        .expect_err("scope mismatch")
        .to_string(),
        "worker event scope does not match route scope"
    );
}

struct RecordingLocalExecutor {
    output: String,
}

impl LocalAgentExecutor for RecordingLocalExecutor {
    fn lease_workspace(
        &self,
        request: LocalAgentWorkspaceRequest,
    ) -> anyhow::Result<LocalAgentWorkspace> {
        Ok(test_local_workspace(&request))
    }

    fn execute(
        &self,
        _request: LocalAgentExecutionRequest,
    ) -> anyhow::Result<LocalAgentExecutionOutput> {
        Ok(LocalAgentExecutionOutput { raw_output: self.output.clone() })
    }
}

struct BlockingLocalExecutor {
    started: Mutex<Option<mpsc::Sender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
    output: String,
}

impl LocalAgentExecutor for BlockingLocalExecutor {
    fn lease_workspace(
        &self,
        request: LocalAgentWorkspaceRequest,
    ) -> anyhow::Result<LocalAgentWorkspace> {
        Ok(test_local_workspace(&request))
    }

    fn execute(
        &self,
        _request: LocalAgentExecutionRequest,
    ) -> anyhow::Result<LocalAgentExecutionOutput> {
        if let Some(started) = self.started.lock().expect("started lock").take() {
            let _ = started.send(());
        }
        self.release.lock().expect("release lock").recv().expect("release executor");
        Ok(LocalAgentExecutionOutput { raw_output: self.output.clone() })
    }
}

fn test_local_workspace(request: &LocalAgentWorkspaceRequest) -> LocalAgentWorkspace {
    let workspace_path = request.project_path.join("worktrees").join(request.task_id.as_str());
    std::fs::create_dir_all(&workspace_path).expect("test workspace dir");
    run_test_git(&workspace_path, &["init"]);
    LocalAgentWorkspace {
        slot_id: format!("slot-{}", request.task_id.as_str()),
        path: workspace_path.to_string_lossy().to_string(),
        branch: format!("vw/task/{}", request.task_id.as_str()),
    }
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    serde_json::from_slice(&body).expect("json response")
}

async fn get_json(app: axum::Router, uri: &str) -> serde_json::Value {
    let response = app
        .oneshot(
            Request::builder().method(Method::GET).uri(uri).body(Body::empty()).expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

async fn post_json<T: serde::Serialize>(
    app: axum::Router,
    uri: &str,
    body: &T,
) -> serde_json::Value {
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(body).expect("request json")))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "unexpected response: {body}");
    body
}

async fn wait_for_log(app: axum::Router, uri: &str, message: &str) -> serde_json::Value {
    for _ in 0..50 {
        let logs = get_json(app.clone(), uri).await;
        if logs["logs"].as_array().expect("logs array").iter().any(|log| log["message"] == message)
        {
            return logs;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("timed out waiting for log message {message}");
}

async fn wait_for_store_log(
    store: &AgentSchedulerStore,
    project_path: &std::path::Path,
    task_id: &TaskId,
    message: &str,
) -> Vec<StoredAgentTaskLog> {
    for _ in 0..50 {
        let logs = store.list_task_logs(project_path, task_id).expect("logs");
        if logs.iter().any(|log| log.message == message) {
            return logs;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("timed out waiting for store log message {message}");
}

async fn wait_for_store_artifact(
    store: &AgentSchedulerStore,
    project_path: &std::path::Path,
    task_id: &TaskId,
    kind: &str,
) -> Vec<StoredTaskArtifact> {
    for _ in 0..50 {
        let artifacts = store.list_task_artifacts(project_path, task_id).expect("artifacts");
        if artifacts.iter().any(|artifact| artifact.kind == kind) {
            return artifacts;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("timed out waiting for store artifact kind {kind}");
}

fn seed_matrix_worker(
    store: &AgentSchedulerStore,
    project_path: &std::path::Path,
    worker_id: &str,
) {
    store
        .upsert_worker_snapshot(
            project_path,
            NewAgentWorkerSnapshot {
                id: WorkerName::new(worker_id).expect("worker id"),
                runtime: Some("matrix".to_string()),
                deployment: Some("local".to_string()),
                model: "auto".to_string(),
                model_provider: None,
                status: AgentWorkerStatus::Idle,
                desired_state: Some("Running".to_string()),
                team_id: None,
                worker_role: Some("worker".to_string()),
                matrix_user_id: Some(format!("@{worker_id}:localhost")),
                room_id: Some("!matrix-room:localhost".to_string()),
                container_state: Some("running".to_string()),
                idle_since_ms: None,
                last_seen_at_ms: Some(TimestampMs(1_783_000_910_000)),
                last_heartbeat_at: None,
                capabilities: vec![AgentCapabilityRecord::skill("matrix-dispatch")],
                updated_at_ms: TimestampMs(1_783_000_910_000),
            },
        )
        .expect("seed matrix worker");
}

fn claim_matrix_run(
    store: &AgentSchedulerStore,
    project_path: &std::path::Path,
    task_id: &str,
    run_id: &str,
    worker_id: &str,
) {
    store
        .claim_task_run(
            project_path,
            crate::agent_scheduler::store::runs::ClaimTaskRunRequest {
                run_id: RunId::new(run_id).expect("run id"),
                task_id: TaskId::new(task_id).expect("task id"),
                worker_id: WorkerName::new(worker_id).expect("worker id"),
                lease_acquired_at_ms: TimestampMs(1_783_000_910_500),
                lease_expires_at_ms: Some(TimestampMs(1_783_000_940_500)),
            },
        )
        .expect("claim active matrix run")
        .expect("active matrix run claimed");
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn scheduler_project_id(project_path: &std::path::Path) -> String {
    let project_id = project_path
        .storage_scope()
        .expect("scheduler test scope")
        .project_scope()
        .project_id()
        .as_ref()
        .to_owned();
    project::store_local_test_project(&project::Info {
        id: project_id.clone(),
        worktree: project_path.to_string_lossy().into_owned(),
        vcs: None,
        name: Some("Agent Scheduler test project".to_owned()),
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    })
    .expect("register scheduler test project");
    project_id
}

fn init_merge_test_repo(project_path: &std::path::Path, source_branch: &str) {
    std::fs::create_dir_all(project_path).expect("project dir");
    run_test_git(project_path, &["init"]);
    run_test_git(project_path, &["config", "user.email", "test@example.com"]);
    run_test_git(project_path, &["config", "user.name", "Test User"]);
    run_test_git(project_path, &["checkout", "-b", "main"]);
    std::fs::write(project_path.join("base.txt"), "base\n").expect("base file");
    run_test_git(project_path, &["add", "base.txt"]);
    run_test_git(project_path, &["commit", "-m", "initial"]);
    run_test_git(project_path, &["checkout", "-b", source_branch]);
    std::fs::write(project_path.join("feature.txt"), "feature\n").expect("feature file");
    run_test_git(project_path, &["add", "feature.txt"]);
    run_test_git(project_path, &["commit", "-m", "feature change"]);
    run_test_git(project_path, &["checkout", "main"]);
}

async fn register_merge_test_project(project_path: &std::path::Path) -> (String, project::Info) {
    let project_id = scheduler_project_id(project_path);
    let info = project::Info {
        id: project_id.clone(),
        worktree: project_path.to_string_lossy().to_string(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    project::store_local_test_project(&info).expect("project write");
    (project_id, info)
}

async fn cleanup_merge_test_project(project_id: &str) {
    project::remove_local_test_project(project_id).expect("project cleanup");
}

fn submitted_merge_task(
    task_id: &str,
    project_path: &std::path::Path,
    source_branch: &str,
    include_worktree_metadata: bool,
) -> NewAgentTask {
    let mut metadata = json!({
        "prompt": "Merge this submitted scheduler task",
        "merge_source_branch": source_branch,
        "merge_target_branch": "main",
    });
    if include_worktree_metadata {
        let metadata = metadata.as_object_mut().expect("metadata object");
        metadata
            .insert("selected_worktree_path".to_string(), json!(project_path.to_string_lossy()));
        metadata.insert("selected_worktree_branch".to_string(), json!(source_branch));
    }
    NewAgentTask {
        id: TaskId::new(task_id).expect("task id"),
        title: "Merge submitted scheduler task".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Submitted,
        spec_uri: format!("shared/tasks/{task_id}/spec.md"),
        meta_uri: format!("shared/tasks/{task_id}/meta.json"),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata,
        created_at_ms: TimestampMs(1_783_000_800_000),
    }
}

async fn post_merge_command(
    app: axum::Router,
    project_id: &str,
    task_id: &str,
) -> Result<axum::response::Response, std::convert::Infallible> {
    let command_uri = format!("/agent-scheduler/projects/{project_id}/commands");
    let command = AgentSchedulerCommandRequest {
        task_id: task_id.into(),
        command: AgentSchedulerCommandKindDto::Merge,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("merge ready".to_string()),
        payload: Default::default(),
    };
    app.oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(command_uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&command).expect("command json")))
            .expect("command request"),
    )
    .await
}

async fn post_json_response<T>(app: axum::Router, uri: &str, body: &T) -> axum::response::Response
where
    T: serde::Serialize,
{
    app.oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(body).expect("request json")))
            .expect("request"),
    )
    .await
    .expect("response")
}

fn workflow_status_with_transitions(
    key: &str,
    role: TaskPoolWorkflowStatusRoleDto,
    transitions: Vec<TaskPoolWorkflowTransitionDto>,
) -> TaskPoolWorkflowStatusDto {
    TaskPoolWorkflowStatusDto {
        key: key.to_string(),
        label: key.to_string(),
        role,
        enabled: true,
        terminal: false,
        next: None,
        transitions,
    }
}

fn workflow_transition(outcome: &str, target: &str) -> TaskPoolWorkflowTransitionDto {
    TaskPoolWorkflowTransitionDto {
        outcome: outcome.to_string(),
        label: None,
        target: target.to_string(),
    }
}

fn run_test_git(repo: &std::path::Path, args: &[&str]) {
    let output = Command::new("git").current_dir(repo).args(args).output().expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn scheduler_wake_concurrency_clamps_to_supported_bounds() {
    assert_eq!(normalize_project_wake_max_concurrent(None), DEFAULT_PROJECT_WAKE_MAX_CONCURRENT);
    assert_eq!(normalize_project_wake_max_concurrent(Some(0)), 1);
    assert_eq!(
        normalize_project_wake_max_concurrent(Some(u32::MAX)),
        MAX_PROJECT_WAKE_MAX_CONCURRENT
    );
}

#[test]
fn scheduler_status_helpers_distinguish_idempotent_and_slot_states() {
    assert!(start_command_is_idempotent_for_status(AgentTaskStatus::Submitted));
    assert!(!execution_slot_status(AgentTaskStatus::Submitted));
    assert!(execution_slot_status(AgentTaskStatus::Acknowledged));
    assert!(!start_command_is_idempotent_for_status(AgentTaskStatus::Queued));
}

#[test]
fn scheduler_lifecycle_defaults_unknown_desired_state_to_running() {
    assert_eq!(lifecycle_from_desired_state(Some("Sleeping")), WorkerLifecycleState::Sleeping);
    assert_eq!(lifecycle_from_desired_state(Some("Stopped")), WorkerLifecycleState::Stopped);
    assert_eq!(lifecycle_from_desired_state(Some("sleeping")), WorkerLifecycleState::Running);
    assert_eq!(lifecycle_from_desired_state(None), WorkerLifecycleState::Running);
}

#[test]
fn scheduler_workspace_context_accepts_nested_and_legacy_metadata() {
    let nested = workspace_context_from_value(&json!({
        "workspace": {"slot_id": "slot-1", "path": "/tmp/work", "branch": "feature"}
    }))
    .expect("nested workspace");
    assert_eq!(nested.slot_id, "slot-1");
    assert_eq!(nested.path, "/tmp/work");
    assert_eq!(nested.branch, "feature");

    let legacy = workspace_context_from_value(&json!({
        "selected_worktree_slot_id": "slot-2",
        "selected_worktree_path": "/tmp/legacy",
        "selected_worktree_branch": "legacy"
    }))
    .expect("legacy workspace");
    assert_eq!(legacy.slot_id, "slot-2");
    assert!(
        workspace_context_from_value(&json!({"workspace": {"slot_id": "slot-only"}})).is_none()
    );
}

#[test]
fn scheduler_metadata_fields_trim_strings_and_reject_numeric_overflow() {
    let metadata = json!({
        "name": "  worker  ",
        "items": [" first ", "", 7, "second"],
        "enabled": true,
        "count": u64::from(u32::MAX) + 1,
        "offset": -12
    });

    assert_eq!(string_field(&metadata, "name").as_deref(), Some("worker"));
    assert_eq!(string_array_field(&metadata, "items"), ["first", "second"]);
    assert_eq!(bool_field(&metadata, "enabled"), Some(true));
    assert_eq!(u32_field(&metadata, "count"), None);
    assert_eq!(i64_field(&metadata, "offset"), Some(-12));
    assert_eq!(optional_text(Some("  text  ".to_string())).as_deref(), Some("text"));
    assert_eq!(optional_text(Some(" \n ".to_string())), None);
}

#[test]
fn scheduler_event_error_prefers_payload_and_requires_error_context_for_message() {
    assert_eq!(
        event_error_message(
            AgentEventKind::RunFailed,
            None,
            "fallback",
            &json!({"error_message": "payload error", "error": "secondary"}),
        )
        .as_deref(),
        Some("payload error")
    );
    assert_eq!(
        event_error_message(AgentEventKind::RunFailed, None, "fallback", &json!({})).as_deref(),
        Some("fallback")
    );
    assert_eq!(
        event_error_message(AgentEventKind::TaskCreated, None, "informational", &json!({})),
        None
    );
}

#[test]
fn scheduler_time_and_merge_helpers_preserve_safety_boundaries() {
    assert_eq!(add_ms_or_saturate(TimestampMs(100), -50), TimestampMs(100));
    assert_eq!(add_ms_or_saturate(TimestampMs(i64::MAX - 1), 10), TimestampMs(i64::MAX));
    assert!(valid_merge_branch(" feature/test "));
    assert!(!valid_merge_branch("HEAD"));
    assert!(!valid_merge_branch("  "));
}
