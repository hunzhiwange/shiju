// 覆盖代理调度 API 的租户隔离、任务分派及生命周期推进。

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
    AgentSchedulerReplaceTaskGraphRequest,
    AgentSchedulerTaskGraphEdgeDto, AgentSchedulerTaskMetadataPatchDto,
    AgentSchedulerTaskMetadataUpdateRequest, AgentTaskKindDto, AgentTaskStatusDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

// 测试使用独立租户、项目和执行器夹具，防止调度状态在作用域之间串扰。
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
async fn agent_scheduler_routes_exclude_removed_remote_resources() {
    let app = router::<()>().with_state(());
    for path in [
        "/agent-scheduler/projects/project-a/workers",
        "/agent-scheduler/projects/project-a/teams",
        "/agent-scheduler/projects/project-a/tasks/task-a/runs/run-a/worker-events",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "unexpected route: {path}");
    }
}

// 失败转换同时检查任务、运行和事件投影，避免只更新其中一个状态视图。
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

// 网关命令测试通过真实存储观察排队与执行结果，避免只验证路由已注册。
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


// 项目唤醒只应选择图依赖已就绪且生命周期允许的任务。
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
async fn project_local_wake_uses_graph_readiness() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-task-graph");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "unexpected local execution".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    let mut prerequisite =
        task_graph_task("task-project-wake-prerequisite", AgentTaskStatus::Submitted);
    prerequisite.metadata = json!({
        "prompt": "Do not run before the graph prerequisite completes",
        "priority": 1,
        "order_no": 1,
        "model": "auto",
        "agent": "main"
    });
    store.create_task(&project_path, prerequisite).expect("seed prerequisite task");
    let mut blocked = task_graph_task("task-project-wake-blocked", AgentTaskStatus::Queued);
    blocked.metadata = json!({
        "prompt": "Do not run while the graph prerequisite is incomplete",
        "priority": 1,
        "order_no": 2,
        "model": "auto",
        "agent": "main"
    });
    store.create_task(&project_path, blocked).expect("seed blocked task");
    store
        .replace_task_dependencies(
            &project_path,
            vec![crate::agent_scheduler::task_graph::AgentTaskDependency::new(
                TaskId::new("task-project-wake-prerequisite").expect("prerequisite task id"),
                TaskId::new("task-project-wake-blocked").expect("blocked task id"),
            )],
        )
        .expect("store graph dependency");

    let project_id = scheduler_project_id(&project_path);
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let wake_uri = format!("/agent-scheduler/projects/{project_id}/wake");
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(wake_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&AgentSchedulerProjectWakeRequest {
                        now_ms: TimestampMs(1_783_000_904_000),
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
    assert_eq!(response_json(response).await["dispatched"], false);
    let task = store
        .load_task(
            &project_path,
            &TaskId::new("task-project-wake-blocked").expect("blocked task id"),
        )
        .expect("load blocked task")
        .expect("blocked task persists");
    assert_eq!(task.status, AgentTaskStatus::Queued);
}

#[tokio::test]
async fn agent_scheduler_project_wake_ignores_autonomous_staff_assignment() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-wake-staff-assign");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "local execution without staff assignment".to_string(),
    });
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
    assert!(task.metadata.get("task_board.staff.member_id").is_none());
    assert_eq!(body["dispatched"], true);
    assert_eq!(body["task_id"], "task-autonomous-staff-assign");
    assert!(store.load_latest_run_for_task(&project_path, &task_id).expect("latest run").is_some());
}

#[tokio::test]
async fn project_wake_does_not_apply_staff_plan() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-wake-without-staff-plan");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(RecordingLocalExecutor {
        output: "local execution without staff plan".to_string(),
    });
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    store
        .create_task(
            &project_path,
            NewAgentTask {
                id: TaskId::new("task-wake-without-staff-plan").expect("task id"),
                title: "Dispatch without applying staff plan".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-wake-without-staff-plan/spec.md".to_string(),
                meta_uri: "shared/tasks/task-wake-without-staff-plan/meta.json".to_string(),
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
                    "prompt": "Run locally without staff orchestration"
                }),
                created_at_ms: TimestampMs(1_783_000_906_500),
            },
        )
        .expect("seed task");

    let response = gateway
        .wake_project(
            scheduler_project_id(&project_path),
            owned_scheduler_context(&project_path).expect("scheduler context"),
            AgentSchedulerProjectWakeRequest {
                now_ms: TimestampMs(1_783_000_906_600),
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                auto_promote_pool_tasks: false,
            },
        )
        .await
        .expect("wake project");

    assert!(response.dispatched);
    let task = store
        .load_task(
            &project_path,
            &TaskId::new("task-wake-without-staff-plan").expect("task id"),
        )
        .expect("load task")
        .expect("task");
    assert!(task.metadata.get("task_board.staff.member_id").is_none());
}

// 任务板是事件投影而非独立真相源，此处固定首次唤醒后的投影同步。
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
async fn agent_scheduler_gateway_policy_wake_promotes_and_dispatches_pool_task() {
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
    assert_eq!(body["dispatched"], true);
    assert_eq!(body["task_id"], "task-policy-pool-wake");

    let task_id = TaskId::new("task-policy-pool-wake").expect("task id");
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_ne!(task.status, AgentTaskStatus::Draft);
    let events = store.list_events_after(&project_path, None, 100).expect("events");
    assert!(events.iter().any(|event| event.payload["policy_action"] == "auto_promote_pool"));
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

// 任务图替换按项目作用域执行，并要求非法图整体拒绝而不是部分落库。
#[tokio::test]
async fn agent_scheduler_gateway_replaces_and_reads_scoped_task_graph() {
    let _ = agent_scheduler_project_task_graph;
    let _ = agent_scheduler_project_replace_task_graph;

    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-task-graph-gateway");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");

    for (task_id, status) in [
        ("task-graph-submitted", AgentTaskStatus::Submitted),
        ("task-graph-completed", AgentTaskStatus::Completed),
        ("task-graph-failed", AgentTaskStatus::Failed),
        ("task-graph-cancelled", AgentTaskStatus::Cancelled),
        ("task-graph-archived", AgentTaskStatus::Archived),
        ("task-graph-waiting", AgentTaskStatus::Queued),
        ("task-graph-ready", AgentTaskStatus::Queued),
        ("task-graph-blocked-failed", AgentTaskStatus::Queued),
        ("task-graph-blocked-cancelled", AgentTaskStatus::Queued),
        ("task-graph-blocked-archived", AgentTaskStatus::Queued),
    ] {
        store
            .create_task(&project_path, task_graph_task(task_id, status))
            .expect("seed task graph task");
    }

    let project_id = scheduler_project_id(&project_path);
    let graph_uri = format!("/agent-scheduler/projects/{project_id}/task-graph");
    let request = AgentSchedulerReplaceTaskGraphRequest {
        edges: vec![
            task_graph_edge("task-graph-submitted", "task-graph-waiting"),
            task_graph_edge("task-graph-completed", "task-graph-ready"),
            task_graph_edge("task-graph-failed", "task-graph-blocked-failed"),
            task_graph_edge("task-graph-cancelled", "task-graph-blocked-cancelled"),
            task_graph_edge("task-graph-archived", "task-graph-blocked-archived"),
        ],
    };
    let response = put_json_response(
        router_with_gateway::<()>(Arc::new(gateway.clone())).with_state(()),
        &graph_uri,
        &request,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let snapshot = response_json(response).await;
    assert_eq!(snapshot["edges"].as_array().expect("edges").len(), 5);
    assert_task_graph_readiness(
        &snapshot,
        "task-graph-waiting",
        "waiting_dependencies",
        "waiting_task_ids",
        "task-graph-submitted",
    );
    assert_task_graph_readiness(&snapshot, "task-graph-ready", "ready", "", "");
    for (task_id, predecessor_id) in [
        ("task-graph-blocked-failed", "task-graph-failed"),
        ("task-graph-blocked-cancelled", "task-graph-cancelled"),
        ("task-graph-blocked-archived", "task-graph-archived"),
    ] {
        assert_task_graph_readiness(
            &snapshot,
            task_id,
            "blocked_by_dependency",
            "blocking_task_ids",
            predecessor_id,
        );
    }

    store
        .transition_task_status(
            &project_path,
            &TaskId::new("task-graph-submitted").expect("task id"),
            AgentTaskStatus::Completed,
            TimestampMs(1_783_001_000_000),
        )
        .expect("complete predecessor");
    let snapshot =
        get_json(router_with_gateway::<()>(Arc::new(gateway)).with_state(()), &graph_uri).await;
    let completed_predecessor = snapshot["tasks"]
        .as_array()
        .expect("graph tasks")
        .iter()
        .find(|task| task["task_id"] == "task-graph-submitted")
        .expect("completed predecessor in graph snapshot");
    assert_eq!(completed_predecessor["status"], "completed");
    assert_task_graph_readiness(&snapshot, "task-graph-waiting", "ready", "", "");
}

#[tokio::test]
async fn agent_scheduler_gateway_rejects_invalid_graph_without_partial_write() {
    let _ = agent_scheduler_project_task_graph;
    let _ = agent_scheduler_project_replace_task_graph;

    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-task-graph-invalid");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = gateway.store.clone();
    store.initialize_project(&project_path).expect("init scheduler");
    for task_id in ["task-graph-before", "task-graph-after"] {
        store
            .create_task(&project_path, task_graph_task(task_id, AgentTaskStatus::Queued))
            .expect("seed task graph task");
    }

    let project_id = scheduler_project_id(&project_path);
    let graph_uri = format!("/agent-scheduler/projects/{project_id}/task-graph");
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let valid = AgentSchedulerReplaceTaskGraphRequest {
        edges: vec![task_graph_edge("task-graph-before", "task-graph-after")],
    };
    let response = put_json_response(app.clone(), &graph_uri, &valid).await;
    assert_eq!(response.status(), StatusCode::OK);

    let invalid = AgentSchedulerReplaceTaskGraphRequest {
        edges: vec![task_graph_edge("task-graph-before", "task-graph-before")],
    };
    let response = put_json_response(app.clone(), &graph_uri, &invalid).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = response_json(response).await;
    assert!(error.to_string().contains("task graph rejects self dependency"));

    let snapshot = get_json(app, &graph_uri).await;
    assert_eq!(
        snapshot["edges"],
        json!([{
            "predecessor_task_id": "task-graph-before",
            "successor_task_id": "task-graph-after",
        }])
    );
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

// 本地启动命令只确认调度已接受，不等待长任务结束，防止网关请求被执行时长阻塞。
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

// 合并测试使用真实 Git 分支和锁，验证默认目标、锁释放及失败诊断形成完整事务边界。
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



#[test]
fn gateway_local_agent_stream_request_forwards_execution_selection() {
    let request = LocalAgentExecutionRequest {
        project_path: PathBuf::from("/tmp/project-root"),
        task_id: TaskId::new("task-deepseek").expect("task id"),
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




// 无作用域路由必须先解析租户和项目上下文，再允许调用任何调度存储方法。
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

fn task_graph_edge(
    predecessor_task_id: &str,
    successor_task_id: &str,
) -> AgentSchedulerTaskGraphEdgeDto {
    AgentSchedulerTaskGraphEdgeDto {
        predecessor_task_id: predecessor_task_id.to_string(),
        successor_task_id: successor_task_id.to_string(),
    }
}

fn task_graph_task(task_id: &str, status: AgentTaskStatus) -> NewAgentTask {
    NewAgentTask {
        id: TaskId::new(task_id).expect("task id"),
        title: format!("Task graph {task_id}"),
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
        metadata: json!({}),
        created_at_ms: TimestampMs(1_783_000_999_000),
    }
}

fn assert_task_graph_readiness(
    snapshot: &serde_json::Value,
    task_id: &str,
    readiness: &str,
    dependency_key: &str,
    dependency_task_id: &str,
) {
    let task = snapshot["tasks"]
        .as_array()
        .expect("graph tasks")
        .iter()
        .find(|task| task["task_id"] == task_id)
        .unwrap_or_else(|| panic!("graph task not found: {task_id}"));
    assert_eq!(task["readiness"], readiness, "unexpected readiness for {task_id}");
    if dependency_key.is_empty() {
        assert!(task.get("waiting_task_ids").is_none());
        assert!(task.get("blocking_task_ids").is_none());
    } else {
        assert_eq!(task[dependency_key], json!([dependency_task_id]));
    }
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

async fn put_json_response<T>(app: axum::Router, uri: &str, body: &T) -> axum::response::Response
where
    T: serde::Serialize,
{
    app.oneshot(
        Request::builder()
            .method(Method::PUT)
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
