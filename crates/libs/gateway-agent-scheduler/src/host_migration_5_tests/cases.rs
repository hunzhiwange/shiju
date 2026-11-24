use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::json;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;
use vw_api_agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentSchedulerBulkCommandRequest,
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyTaskImportDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerReplaceTaskGraphRequest, AgentSchedulerTaskGraphEdgeDto,
    AgentSchedulerTaskMetadataPatchDto, AgentSchedulerTaskMetadataUpdateRequest, AgentTaskKindDto,
    AgentTaskStatusDto,
};
use vw_api_transport::common::TimestampMs;
use vw_api_task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

// 测试使用独立租户、项目和执行器夹具，防止调度状态在作用域之间串扰。


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
            placement: None,
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
    acceptance_gateway_task(false).await;
}

#[tokio::test]
async fn task_pool_acceptance_non_git_runs_through_gateway_without_git() {
    acceptance_gateway_task(false).await;
}

#[tokio::test]
async fn task_pool_acceptance_git_task_has_zero_implicit_actions() {
    acceptance_gateway_task(true).await;
}

struct AcceptanceGatewayExecutor {
    output: String,
}

impl LocalAgentExecutor for AcceptanceGatewayExecutor {
    fn lease_workspace(
        &self,
        request: LocalAgentWorkspaceRequest,
    ) -> anyhow::Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            path: request.project_path.display().to_string(),
            slot_id: None,
            branch: None,
        })
    }

    fn execute(
        &self,
        request: LocalAgentExecutionRequest,
    ) -> anyhow::Result<LocalAgentExecutionOutput> {
        std::fs::write(
            std::path::Path::new(&request.workspace.path).join("output.txt"),
            "gateway artifact",
        )?;
        Ok(LocalAgentExecutionOutput { raw_output: self.output.clone() })
    }
}

async fn acceptance_gateway_task(git: bool) {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-agent-scheduler-gateway");
    std::fs::create_dir_all(&project_path).expect("project dir");
    if git {
        init_merge_test_repo(&project_path, "acceptance-base");
        std::fs::write(project_path.join("prestaged.txt"), "keep staged").unwrap();
        let output = shell_command::git_std_command()
            .current_dir(&project_path)
            .args(["add", "prestaged.txt"])
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    let git_state = || {
        [
            vec!["rev-parse", "HEAD"],
            vec!["diff", "--cached"],
            vec!["worktree", "list", "--porcelain"],
        ]
        .map(|args| {
            let output = shell_command::git_std_command()
                .current_dir(&project_path)
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success());
            output.stdout
        })
    };
    let before = git.then(&git_state);
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
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(AcceptanceGatewayExecutor {
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
    let logs =
        wait_for_log(app.clone(), &logs_uri, "local execution result advanced lifecycle").await;
    assert_eq!(logs["task_id"], "task-gateway-new");
    assert!(
        logs["logs"]
            .as_array()
            .expect("logs array")
            .iter()
            .any(|log| log["message"] == "local execution result advanced lifecycle")
    );
    let task =
        store.load_task(&project_path, &task_id).expect("load scheduler task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);

    let snapshot =
        get_json(app.clone(), &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert_eq!(snapshot["tasks"][0]["task_id"], "task-gateway-new");
    assert_eq!(snapshot["tasks"][0]["status"], "completed");
    assert_eq!(snapshot["tasks"][0]["status_key"], "completed");

    let artifacts = get_json(
        app,
        &format!("/agent-scheduler/projects/{project_id}/tasks/task-gateway-new/artifacts"),
    )
    .await;
    assert_eq!(artifacts["task_id"], "task-gateway-new");
    let result = artifacts["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|artifact| artifact["kind"] == "execution_result")
        .expect("execution result");
    assert_eq!(result["payload"]["success"], true);
    assert_eq!(result["payload"]["output"], "mock local execution output");
    assert_eq!(
        std::fs::read_to_string(project_path.join("output.txt")).unwrap(),
        "gateway artifact"
    );
    assert!(!project_path.join("worktrees").exists());
    if let Some(before) = before {
        assert_eq!(git_state(), before, "ordinary task must preserve HEAD, index and worktrees");
    } else {
        assert!(!project_path.join(".git").exists());
    }
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
            vec![task_scheduler::task_graph::AgentTaskDependency::new(
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
    wait_for_store_log(
        &store,
        &project_path,
        &task_id,
        "local execution result advanced lifecycle",
    )
    .await;
    let task = store.load_task(&project_path, &task_id).expect("load task").expect("task");
    assert_eq!(task.status, AgentTaskStatus::Completed);
    assert_eq!(task.metadata["task_board.status_key"], "completed");
    assert_eq!(task.metadata["task_board.status_role"], "done");

    let snapshot = get_json(app, &format!("/agent-scheduler/projects/{project_id}/tasks")).await;
    assert_eq!(snapshot["tasks"][0]["task_id"], "task-project-wake-projection");
    assert_eq!(snapshot["tasks"][0]["status"], "completed");
    assert_eq!(snapshot["tasks"][0]["status_key"], "completed");
    assert_eq!(snapshot["tasks"][0]["read_model"]["status_role"], "done");
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
            task_scheduler::store::runs::ClaimTaskRunRequest {
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
    let holder = gateway_task::task_pool::task_worktree_pool_service()
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
            .any(|log| log["message"] == "local execution result advanced lifecycle")
    );

    release_tx.send(()).expect("release executor");
    let submitted_logs =
        wait_for_log(app, &logs_uri, "local execution result advanced lifecycle").await;
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

// 合并测试使用真实 Git 分支和锁，验证默认目标、锁释放及失败诊断形成完整事务边界。



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
        reasoning_effort: Some(vw_api_chat_identity::ReasoningEffortDto::High),
        workspace: task_scheduler::runner::local::LocalAgentWorkspace {
            slot_id: Some("slot-deepseek".to_string()),
            path: "/tmp/project-root/worktrees/slot-deepseek".to_string(),
            branch: Some("codex/task-deepseek".to_string()),
        },
    };

    let body = gateway_local_agent_stream_request(&request);

    assert_eq!(body.model.as_deref(), Some("deepseek/deepseek-v4-flash"));
    assert_eq!(body.agent, None);
    assert_eq!(body.reasoning_effort, Some(vw_api_chat_identity::ReasoningEffortDto::High));
    assert_eq!(body.messages[0]["content"], "Use the selected model");
    let options = body.options.expect("gateway options");
    assert_eq!(options["cwd"], "/tmp/project-root/worktrees/slot-deepseek");
    assert_eq!(options["acp_test"], true);
    assert_eq!(options["acp_agent"], "codex");
    assert_eq!(options["acp_permission_mode"], "approve-reads");
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
            task_scheduler::store::runs::ClaimTaskRunRequest {
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

fn default_context() -> AgentSchedulerContext {
    let tenant = TenantScope::new("default-platform", "default-company").expect("tenant");
    let scope = ProjectScope::new(tenant, ProjectId::from("default-project")).expect("scope");
    AgentSchedulerContext::new(AgentSchedulerScope::new(scope), "/tmp/default-project")
        .expect("context")
}

fn create_task_request(task_id: &str) -> AgentSchedulerCreateTaskRequest {
    AgentSchedulerCreateTaskRequest {
        placement: None,
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
        slot_id: Some(format!("slot-{}", request.task_id.as_str())),
        path: workspace_path.to_string_lossy().to_string(),
        branch: Some(format!("vw/task/{}", request.task_id.as_str())),
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
    assert_eq!(nested.slot_id.as_deref(), Some("slot-1"));
    assert_eq!(nested.path, "/tmp/work");
    assert_eq!(nested.branch.as_deref(), Some("feature"));

    let legacy = workspace_context_from_value(&json!({
        "selected_worktree_slot_id": "slot-2",
        "selected_worktree_path": "/tmp/legacy",
        "selected_worktree_branch": "legacy"
    }))
    .expect("legacy workspace");
    assert_eq!(legacy.slot_id.as_deref(), Some("slot-2"));
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
    assert!(valid_merge_branch("feature/test"));
    assert!(!valid_merge_branch("HEAD"));
    assert!(!valid_merge_branch(" "));
    assert_eq!(add_ms_or_saturate(TimestampMs(100), -50), TimestampMs(100));
    assert_eq!(add_ms_or_saturate(TimestampMs(i64::MAX - 1), 10), TimestampMs(i64::MAX));
}

#[tokio::test]
async fn task_pool_schedule_status_named_merge_does_not_execute_git() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("project");
    init_merge_test_repo(&path, "feature/explicit-merge");
    let (project_id, _) = register_merge_test_project(&path).await;
    let gateway = StoreBackedAgentSchedulerGateway::default();
    gateway.store.initialize_project(&path).unwrap();
    let task = submitted_merge_task("explicit-merge", &path, "feature/explicit-merge", true);
    let task = gateway.store.create_task(&path, task).unwrap();
    let store = gateway.store.clone();
    let app = router_with_gateway::<()>(Arc::new(gateway)).with_state(());
    let response = post_merge_command(app, &project_id, "explicit-merge").await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await.to_string().contains("explicit workflow"));
    assert_eq!(
        store.load_task(&path, &task.id).unwrap().unwrap().status,
        AgentTaskStatus::Submitted
    );
    assert!(!path.join("feature.txt").exists());
    assert!(store.list_task_artifacts(&path, &task.id).unwrap().is_empty());
    assert!(
        store
            .list_task_logs(&path, &task.id)
            .unwrap()
            .iter()
            .any(|l| l.message.contains("explicit workflow"))
    );
    cleanup_merge_test_project(&project_id).await;
}

#[tokio::test]
async fn work_plan_gateway_placement_create_edit_and_snapshot() {
    let project = tempfile::tempdir().expect("project");
    let project_id = scheduler_project_id(project.path());
    let app = router_with_gateway::<()>(Arc::new(StoreBackedAgentSchedulerGateway::default()))
        .with_state(());
    let tasks = format!("/agent-scheduler/projects/{project_id}/tasks");
    let parent = serde_json::to_value(create_task_request("plan-parent")).expect("request");
    assert!(parent.get("placement").is_none());
    let response = post_json_response(app.clone(), &tasks, &parent).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_json(response).await["task"].get("placement").is_none());

    let mut child = serde_json::to_value(create_task_request("plan-child")).expect("request");
    child["placement"] = json!({"kind": "child", "parent_task_id": "plan-parent"});
    child["status"] = json!("completed");
    child["metadata"] = json!({"prompt": "Implement child task", "acp_agent": "codex"});
    let response = post_json_response(app.clone(), &tasks, &child).await;
    assert_eq!(response.status(), StatusCode::OK);
    let created = response_json(response).await;
    assert_eq!(created["task"]["placement"]["batch"], 1);
    assert_ne!(created["task"]["status"], "completed");

    let parent_plan = format!("{tasks}/plan-parent/placement");
    let set_root = json!({
        "placement": {"op": "set", "placement": {"kind": "root", "outer_batch": 2, "inner_batch": 3}}
    });
    let response = put_json_response(app.clone(), &parent_plan, &set_root).await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = put_json_response(app.clone(), &parent_plan, &json!({})).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["placement"]["inner_batch"], 3);
    let response =
        put_json_response(app.clone(), &parent_plan, &json!({"placement": {"op": "clear"}})).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_json(response).await.get("placement").is_none());

    let child_plan = format!("{tasks}/plan-child/placement");
    let response = put_json_response(app.clone(), &child_plan, &json!({
        "placement": {"op": "set", "placement": {
            "kind": "child", "parent_task_id": "plan-parent", "batch": 4
        }}
    })).await;
    assert_eq!(response.status(), StatusCode::OK);
    let metadata_uri = format!("{tasks}/plan-child/metadata");
    for (patch, expected_status) in [
        (json!({"status_key": "completed", "status": "completed"}), StatusCode::CONFLICT),
        (json!({"metadata_merge": {"task_plan.waiting": true}}), StatusCode::BAD_REQUEST),
    ] {
        let response = app.clone().oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(&metadata_uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"patch": patch}).to_string()))
                .expect("patch request"),
        ).await.expect("patch response");
        assert_eq!(response.status(), expected_status);
    }

    let project_uri = format!("/agent-scheduler/projects/{project_id}");
    let response = post_json_response(
        app.clone(),
        &format!("{project_uri}/commands/bulk"),
        &json!({"commands": [
            {"task_id": "plan-child", "command": "archive"},
            {"task_id": "plan-parent", "command": "soft_delete"}
        ]}),
    ).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["rejected"], 2);

    let events = get_json(app.clone(), &format!("{project_uri}/events")).await;
    let child_events: Vec<_> = events["events"].as_array().expect("events").iter()
        .filter(|event| event["task_id"] == "plan-child").collect();
    assert_eq!(child_events.len(), 2);
    assert_eq!(child_events[0]["kind"], "task_created");
    assert_eq!(child_events[1]["payload"]["task_plan"]["batch"], 4);
    let snapshot = get_json(app, &tasks).await;
    let child = snapshot["tasks"].as_array().expect("tasks").iter()
        .find(|task| task["task_id"] == "plan-child").expect("child");
    assert_eq!(child["placement"], json!({"kind": "child", "parent_task_id": "plan-parent", "batch": 4}));
    assert_eq!(child["prompt"], "Implement child task");
    assert_eq!(child["acp_agent"], "codex");
}

#[tokio::test]
async fn work_plan_gateway_placement_scope_and_retry() {
    let first = tempfile::tempdir().expect("first project");
    let second = tempfile::tempdir().expect("second project");
    let first_id = scheduler_project_id(first.path());
    let second_id = scheduler_project_id(second.path());
    let app = router_with_gateway::<()>(Arc::new(StoreBackedAgentSchedulerGateway::default()))
        .with_state(());
    let first_tasks = format!("/agent-scheduler/projects/{first_id}/tasks");
    let second_tasks = format!("/agent-scheduler/projects/{second_id}/tasks");
    let response = post_json_response(app.clone(), &first_tasks, &create_task_request("plan-parent")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut child = serde_json::to_value(create_task_request("plan-child")).expect("request");
    child["placement"] = json!({"kind": "child", "parent_task_id": "plan-parent", "batch": 2});

    let response = post_json_response(app.clone(), &second_tasks, &child).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let second_board = get_json(app.clone(), &second_tasks).await;
    assert_eq!(second_board["tasks"], json!([]));
    let response = put_json_response(
        app.clone(), &format!("{second_tasks}/plan-parent/placement"), &json!({})
    ).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    for _ in 0..2 {
        let response = post_json_response(app.clone(), &first_tasks, &child).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["task"]["placement"], child["placement"]);
    }
    child["placement"]["batch"] = json!(3);
    let response = post_json_response(app.clone(), &first_tasks, &child).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let response = post_json_response(app.clone(), &first_tasks, &create_task_request("plan-child")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["task"]["placement"]["batch"], 2);
    let snapshot = get_json(app, &first_tasks).await;
    let tasks = snapshot["tasks"].as_array().expect("tasks");
    assert_eq!(tasks.len(), 2);
    let child = tasks.iter().find(|task| task["task_id"] == "plan-child").expect("child");
    assert_eq!(child["placement"]["batch"], 2);
}

fn seed_rolling_tasks(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    tasks: &[(&str, u32, AgentTaskStatus)],
) -> Result<()> {
    use task_scheduler::store::task_plan::TaskPlanPlacement;

    for (index, &(id, batch, status)) in tasks.iter().enumerate() {
        store.create_task_with_plan(
            context,
            NewAgentTask {
                id: TaskId::new(id)?,
                title: id.into(),
                kind: AgentTaskKind::Finite,
                status,
                spec_uri: format!("shared/tasks/{id}/spec.md"),
                meta_uri: format!("shared/tasks/{id}/meta.json"),
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
                metadata: json!({"order_no": index}),
                created_at_ms: TimestampMs(1),
            },
            TaskPlanPlacement::Root { outer_batch: 1, inner_batch: batch },
        )?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_plan_rolling_refills_before_sibling_finishes() -> Result<()> {
    use task_scheduler::domain::AgentRunStatus;
    use task_scheduler::store::runs::{AgentRunFinishStatus, ClaimReadyTaskRunRequest};

    let project = tempfile::tempdir()?;
    let context = owned_scheduler_context(project.path())?;
    let project_id = scheduler_project_id(project.path());
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let gateway = StoreBackedAgentSchedulerGateway::with_executor(super::host::GatedLocalExecutor {
        started: started_tx,
        releases: Mutex::new(release_rx),
    });
    let store = &gateway.store;
    seed_rolling_tasks(store, &context, &[
        ("slow-sibling", 1, AgentTaskStatus::Queued),
        ("fast-first", 1, AgentTaskStatus::Queued),
        ("same-batch-refill", 1, AgentTaskStatus::Queued),
        ("next-batch", 2, AgentTaskStatus::Queued),
    ])?;
    // Hold the sibling's lease separately so the local writer keeps the directory exclusive.
    let slow = store.claim_ready_task_run(&context, ClaimReadyTaskRunRequest {
        run_id: RunId::new("slow-sibling-run")?,
        task_id: TaskId::new("slow-sibling")?,
        lease_acquired_at_ms: TimestampMs(1),
        lease_expires_at_ms: None,
    })?.expect("sibling leased");
    let wake = gateway.wake_project(project_id.clone(), context.clone(), AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(2),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: Some(2),
        auto_promote_pool_tasks: false,
    }).await?;
    assert_eq!(wake.task_id.as_deref(), Some("fast-first"));
    assert_eq!(wake.dispatched_runs.len(), 1);
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(10))?.as_str(), "fast-first");
    release_tx.send(())?;
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(10))?.as_str(), "same-batch-refill");

    let board = gateway.task_board_status(project_id, context.clone()).await?;
    let board_task = |id: &str| board.tasks.iter().find(|task| task.task_id.as_ref() == id).unwrap();
    assert_eq!(board_task("fast-first").status, Some(AgentTaskStatusDto::Completed));
    assert_eq!(board_task("next-batch").status, Some(AgentTaskStatusDto::Queued));
    assert!(matches!(
        &board_task("next-batch").read_model.dispatch_blocker,
        Some(vw_api_agent_scheduler::AgentSchedulerDispatchBlockerDto::WaitingForEarlierBatch { task_ids })
            if task_ids.iter().any(|id| id == "slow-sibling")
                && task_ids.iter().any(|id| id == "same-batch-refill")
    ));
    assert_eq!(store.load_run(&context, &slow.id)?.unwrap().status, AgentRunStatus::Leased);
    assert!(store.load_latest_run_for_task(&context, &TaskId::new("next-batch")?)?.is_none());
    for status in [
        AgentTaskStatus::Dispatched,
        AgentTaskStatus::Acknowledged,
        AgentTaskStatus::Running,
        AgentTaskStatus::Completed,
    ] {
        store.transition_task_status(&context, &slow.task_id, status, TimestampMs(3))?;
    }
    store.finish_run(&context, &slow.id, AgentRunFinishStatus::Succeeded, TimestampMs(4), None)?
        .expect("sibling run completed");
    release_tx.send(())?;
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(10))?.as_str(), "next-batch");
    release_tx.send(())?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_plan_rolling_concurrent_wakes_respect_capacity() -> Result<()> {
    let project = tempfile::tempdir()?;
    let context = owned_scheduler_context(project.path())?;
    let project_id = scheduler_project_id(project.path());
    let first = StoreBackedAgentSchedulerGateway::default();
    let second = StoreBackedAgentSchedulerGateway::default();
    let ids = ["concurrent-a", "concurrent-b", "concurrent-c", "concurrent-d"];
    let tasks = ids.map(|id| (id, 1, AgentTaskStatus::Queued));
    seed_rolling_tasks(&first.store, &context, &tasks)?;
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let wake = |gateway: StoreBackedAgentSchedulerGateway| {
        let context = context.clone();
        let project_id = project_id.clone();
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            barrier.wait().await;
            gateway.wake_project(project_id, context, AgentSchedulerProjectWakeRequest {
                now_ms: TimestampMs(2),
                dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
                max_concurrent: Some(2),
                auto_promote_pool_tasks: false,
            }).await
        })
    };
    let (left, right) = tokio::join!(wake(first.clone()), wake(second));
    let responses = [left??, right??];
    for response in &responses {
        assert_eq!(response.task_id.as_ref(), response.dispatched_runs.first().map(|run| &run.task_id));
        assert_eq!(response.run_id.as_ref(), response.dispatched_runs.first().map(|run| &run.run_id));
    }
    assert_eq!(responses.iter().map(|response| response.dispatched_runs.len()).sum::<usize>(), 2);
    for (index, id) in ids.into_iter().enumerate() {
        let run = first.store.load_latest_run_for_task(&context, &TaskId::new(id)?)?;
        if index < 2 {
            assert_eq!(run.expect("run in available slot").status, task_scheduler::domain::AgentRunStatus::Leased);
        } else {
            assert!(run.is_none());
        }
    }
    Ok(())
}

#[tokio::test]
async fn work_plan_rolling_periodic_wake_refills_after_cancelled_lease_recovery() -> Result<()> {
    use task_scheduler::domain::AgentRunStatus;
    use task_scheduler::engine::recovery::DEFAULT_RECOVERY_GRACE_MS;
    use task_scheduler::store::runs::ClaimReadyTaskRunRequest;

    let project = tempfile::tempdir()?;
    let context = owned_scheduler_context(project.path())?;
    let project_id = scheduler_project_id(project.path());
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let store = &gateway.store;
    seed_rolling_tasks(store, &context, &[
        ("cancelled-owner", 1, AgentTaskStatus::Queued),
        ("same-batch-ready", 1, AgentTaskStatus::Queued),
    ])?;
    let task_id = TaskId::new("cancelled-owner")?;
    let run_id = RunId::new("cancelled-owner-run")?;
    store.claim_ready_task_run(&context, ClaimReadyTaskRunRequest {
        run_id: run_id.clone(),
        task_id: task_id.clone(),
        lease_acquired_at_ms: TimestampMs(2),
        lease_expires_at_ms: Some(TimestampMs(3)),
    })?.expect("initial lease");
    store.transition_task_status(&context, &task_id, AgentTaskStatus::Cancelled, TimestampMs(3))?;

    let wake = |now_ms| gateway.wake_project(
        project_id.clone(), context.clone(), AgentSchedulerProjectWakeRequest {
            now_ms: TimestampMs(now_ms),
            dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
            max_concurrent: Some(1),
            auto_promote_pool_tasks: false,
        },
    );
    assert!(!wake(4).await?.dispatched);
    let response = wake(5 + DEFAULT_RECOVERY_GRACE_MS).await?;
    assert_eq!(response.task_id.as_deref(), Some("same-batch-ready"));
    assert_eq!(store.load_run(&context, &run_id)?.unwrap().status, AgentRunStatus::Cancelled);
    Ok(())
}

#[tokio::test]
async fn work_plan_publish_atomic_and_idempotent() -> Result<()> {
    use task_scheduler::domain::AgentRunStatus;
    use task_scheduler::store::runs::ClaimReadyTaskRunRequest;

    let project = tempfile::tempdir()?;
    let context = owned_scheduler_context(project.path())?;
    let project_id = scheduler_project_id(project.path());
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let parent_id = TaskId::new("publish-parent")?;
    let run_id = RunId::new("publish-planning-run")?;
    seed_rolling_tasks(&gateway.store, &context, &[("publish-parent", 1, AgentTaskStatus::Queued)])?;
    gateway.store.claim_ready_task_run(&context, ClaimReadyTaskRunRequest {
        run_id: run_id.clone(),
        task_id: parent_id.clone(),
        lease_acquired_at_ms: TimestampMs(2),
        lease_expires_at_ms: None,
    })?.expect("parent slot");
    for status in [AgentTaskStatus::Dispatched, AgentTaskStatus::Acknowledged, AgentTaskStatus::Running] {
        gateway.store.transition_task_status(&context, &parent_id, status, TimestampMs(3))?;
    }

    let app = router_with_gateway::<()>(Arc::new(gateway.clone())).with_state(());
    let route = format!("/agent-scheduler/projects/{project_id}/tasks/publish-parent/work-plan");
    let mut request = json!({
        "dispatch_mode": "auto",
        "max_concurrent": 1,
        "plan": {
            "source": "generated",
            "planning_attempt_id": run_id.as_str(),
            "subtasks": [
                {"title":"first", "boundary":"src", "acceptance_criteria":["check"],
                 "target_files":["src/lib.rs"], "batch":1, "depends_on_batches":[],
                 "execution_prompt":"Implement the first step"},
                {"title":"second", "boundary":"src", "acceptance_criteria":["check"],
                 "target_files":["./src/lib.rs"], "batch":1, "depends_on_batches":[],
                 "execution_prompt":"Implement the second step"}
            ],
        },
    });
    let response = post_json_response(app.clone(), &route, &request).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(gateway.store.list_task_plan_children(&context, &parent_id)?.is_empty());
    assert_eq!(gateway.store.load_run(&context, &run_id)?.unwrap().status, AgentRunStatus::Leased);

    request["plan"]["subtasks"][1]["batch"] = json!(2);
    request["plan"]["subtasks"][1]["depends_on_batches"] = json!([1]);
    let (left, right) = tokio::join!(
        post_json_response(app.clone(), &route, &request),
        post_json_response(app.clone(), &route, &request),
    );
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);
    let left = response_json(left).await;
    assert_eq!(left, response_json(right).await);
    let child_ids = left["child_task_ids"].as_array().expect("published child IDs");
    assert_eq!(child_ids.len(), 2);
    let first = TaskId::new(child_ids[0].as_str().expect("first child ID"))?;
    let second = TaskId::new(child_ids[1].as_str().expect("second child ID"))?;
    assert_eq!(gateway.store.load_task(&context, &first)?.unwrap().title, "first");
    assert_eq!(gateway.store.load_task(&context, &second)?.unwrap().title, "second");
    assert_eq!(gateway.store.load_task(&context, &parent_id)?.unwrap().status, AgentTaskStatus::Blocked);
    assert_eq!(gateway.store.load_run(&context, &run_id)?.unwrap().status, AgentRunStatus::Succeeded);
    assert_eq!(gateway.store.list_task_plan_children(&context, &parent_id)?.len(), 2);

    request["plan"]["subtasks"][0]["title"] = json!("changed");
    assert_eq!(post_json_response(app.clone(), &route, &request).await.status(), StatusCode::CONFLICT);
    let wake = gateway.wake_project(project_id, context.clone(), AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(5),
        dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
        max_concurrent: Some(1),
        auto_promote_pool_tasks: false,
    }).await?;
    assert!(wake.task_id.as_deref() == Some(first.as_str())
        || gateway.store.load_latest_run_for_task(&context, &first)?.is_some());
    assert!(gateway.store.load_latest_run_for_task(
        &context, &second,
    )?.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_plan_parent_completion_enqueues_summary_once() -> Result<()> {
    let project = tempfile::tempdir()?;
    let context = owned_scheduler_context(project.path())?;
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let (parent, children) = seed_parent_completion_plan(&gateway.store, &context)?;
    let mut completions = Vec::new();
    for child in children {
        let store = gateway.store.clone();
        let context = context.clone();
        completions.push(tokio::task::spawn_blocking(move || -> Result<usize> {
            set_parent_completion_child(&store, &context, &child, AgentTaskStatus::Completed, "completed")?;
            store.queue_completed_work_plans(&context, TimestampMs(2))
        }));
    }
    let mut queued = 0;
    for completion in completions {
        queued += completion.await??;
    }
    assert_eq!(queued, 1);
    let wake = |now| gateway.wake_project(scheduler_project_id(project.path()), context.clone(),
        AgentSchedulerProjectWakeRequest {
            now_ms: TimestampMs(now),
            dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
            max_concurrent: Some(1),
            auto_promote_pool_tasks: false,
        });
    let first = wake(3).await?;
    assert_eq!(first.task_id.as_deref(), Some(parent.as_str()));
    assert!(wake(4).await?.dispatched_runs.is_empty());
    let run = gateway.store.load_latest_run_for_task(&context, &parent)?.unwrap();
    assert_eq!(run.attempt, 1);
    let events = gateway.store.list_events_after(&context, None, 50)?;
    assert_eq!(events.iter().filter(|event| event.message == "work plan summary queued").count(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_plan_parent_completion_waits_for_success_and_recovers() -> Result<()> {
    let project = tempfile::tempdir()?;
    let context = owned_scheduler_context(project.path())?;
    let project_id = scheduler_project_id(project.path());
    let gateway = StoreBackedAgentSchedulerGateway::default();
    let (parent, children) = seed_parent_completion_plan(&gateway.store, &context)?;
    let wake = gateway.wake_project(project_id.clone(), context.clone(), AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(2),
        dispatch_mode: AgentSchedulerDispatchModeDto::Auto,
        max_concurrent: Some(1),
        auto_promote_pool_tasks: false,
    }).await?;
    assert_eq!(wake.dispatched_runs.len(), 1);
    let child_id = TaskId::new(wake.task_id.unwrap())?;
    assert!(children.contains(&child_id));
    let child_run = gateway.store.load_latest_run_for_task(&context, &child_id)?.unwrap();
    let artifact = gateway.store.record_task_artifact(
        &context,
        task_scheduler::store::artifacts::RecordTaskArtifactRequest {
            task_id: child_id.clone(),
            run_id: Some(child_run.id.clone()),
            kind: "execution_result".into(),
            name: "execution-result.json".into(),
            path: Some("/private/other-worktree/artifact".into()),
            payload: json!({"success": true, "output": "child report for summary"}),
            created_at_ms: TimestampMs(3),
        },
    )?;
    gateway.store.finish_run(
        &context, &child_run.id, task_scheduler::store::runs::AgentRunFinishStatus::Succeeded,
        TimestampMs(3), None,
    )?;
    for child in &children {
        set_parent_completion_child(&gateway.store, &context, child, AgentTaskStatus::Submitted, "code_complete")?;
    }
    assert_eq!(gateway.store.queue_completed_work_plans(&context, TimestampMs(4))?, 0);
    let board = gateway.task_board_status(project_id.clone(), context.clone()).await?;
    let parent_row = board.tasks.iter().find(|task| task.task_id.as_ref() == parent.as_str()).unwrap();
    assert!(matches!(
        &parent_row.read_model.dispatch_blocker,
        Some(vw_api_agent_scheduler::AgentSchedulerDispatchBlockerDto::WorkPlanChildrenIncomplete { children })
            if children.len() == 2 && children.iter().all(|child| child.status_key == "code_complete")
    ));

    // The parent uses the current project's review transition when it summarizes.
    let mut workflow = task_scheduler::lifecycle_bridge::builtin_task_workflow_snapshot();
    let review = TaskPoolWorkflowStatusDto {
        key: "summary_review".into(),
        label: "Summary review".into(),
        role: TaskPoolWorkflowStatusRoleDto::ReviewReady,
        enabled: true,
        terminal: false,
        next: Some("completed".into()),
        transitions: Vec::new(),
    };
    let running = workflow.statuses.iter_mut().find(|status| status.key == "running").unwrap();
    running.next = Some(review.key.clone());
    running.transitions.retain(|transition| transition.outcome != "success");
    running.transitions.push(TaskPoolWorkflowTransitionDto {
        outcome: "success".into(), label: Some("Submit".into()), target: review.key.clone(),
    });
    workflow.statuses.push(review);
    gateway.store.save_task_board_workflow(&context, &workflow)?;
    for child in &children {
        set_parent_completion_child(&gateway.store, &context, child, AgentTaskStatus::Completed, "completed")?;
    }
    assert_eq!(gateway.store.load_task(&context, &parent)?.unwrap().status, AgentTaskStatus::Blocked);

    let (request_tx, request_rx) = mpsc::channel();
    let reloaded = StoreBackedAgentSchedulerGateway::with_executor(ParentSummaryExecutor(request_tx));
    let wake = reloaded.wake_project(project_id, context.clone(), AgentSchedulerProjectWakeRequest {
        now_ms: TimestampMs(5),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: Some(1),
        auto_promote_pool_tasks: false,
    }).await?;
    assert_eq!(wake.task_id.as_deref(), Some(parent.as_str()));
    let request = request_rx.recv_timeout(Duration::from_secs(10))?;
    assert_eq!(request.task_id, parent);
    assert!(request.prompt.contains(child_id.as_str()));
    assert!(request.prompt.contains("child report for summary"));
    assert!(request.prompt.contains(&format!(r#""artifact_id":{}"#, artifact.id)));
    assert!(!request.prompt.contains("/private/other-worktree"));

    let run = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let run = reloaded.store.load_latest_run_for_task(&context, &parent)?.unwrap();
            if run.finished_at_ms.is_some() {
                break Ok::<_, anyhow::Error>(run);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await??;
    assert_eq!(run.status, task_scheduler::domain::AgentRunStatus::Succeeded);
    let task = reloaded.store.load_task(&context, &parent)?.unwrap();
    assert_eq!(task.status, AgentTaskStatus::Submitted);
    assert_eq!(task.metadata["task_board.status_key"], "summary_review");
    Ok(())
}

struct ParentSummaryExecutor(mpsc::Sender<LocalAgentExecutionRequest>);

impl LocalAgentExecutor for ParentSummaryExecutor {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            path: request.project_path.to_string_lossy().into_owned(),
            slot_id: None,
            branch: None,
        })
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        self.0.send(request)?;
        Ok(LocalAgentExecutionOutput { raw_output: "summary ready for review".into() })
    }
}

fn seed_parent_completion_plan(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
) -> Result<(TaskId, [TaskId; 2])> {
    use task_scheduler::store::task_plan::TaskPlanPlacement;
    use vw_api_agent_scheduler::WorkPlanPublication;

    seed_rolling_tasks(store, context, &[("summary-parent", 1, AgentTaskStatus::Queued)])?;
    let parent = TaskId::new("summary-parent")?;
    let parent_task = store.load_task(context, &parent)?.unwrap();
    let children = [TaskId::new("summary-first")?, TaskId::new("summary-second")?];
    for child in &children {
        store.create_task_with_plan(context, NewAgentTask {
            id: child.clone(),
            title: child.as_str().to_owned(),
            kind: AgentTaskKind::Finite,
            status: AgentTaskStatus::Queued,
            spec_uri: format!("shared/tasks/{}/spec.md", child.as_str()),
            meta_uri: format!("shared/tasks/{}/meta.json", child.as_str()),
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
            metadata: parent_task.metadata.clone(),
            created_at_ms: TimestampMs(1),
        }, TaskPlanPlacement::Child { parent_task_id: parent.clone(), batch: 1 })?;
    }
    store.publish_work_plan(context, &parent, &WorkPlanPublication::Manual, TimestampMs(1))?;
    Ok((parent, children))
}

fn set_parent_completion_child(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    child: &TaskId,
    status: AgentTaskStatus,
    status_key: &str,
) -> Result<()> {
    store.update_task_board_task(context, child, task_scheduler::store::tasks::AgentTaskBoardUpdate {
        title: None,
        status: Some(status),
        metadata: json!({"task_board.status_key": status_key}),
        updated_at_ms: TimestampMs(2),
    })?.expect("child task");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_plan_e2e_two_level_root_batches_and_children() -> Result<()> {
    let home = tempfile::tempdir()?;
    let path = home.path().join("scheduler.sqlite");
    exercise_persisted_work_plan(|| agent_database::GlobalDatabase::new(&path)).await?;

    let postgres = agent_database::test_support::postgres_test_database!();
    let result = exercise_persisted_work_plan(|| postgres.reopen()).await;
    postgres.cleanup()?;
    result
}

async fn exercise_persisted_work_plan(
    open_database: impl Fn() -> Result<agent_database::GlobalDatabase>,
) -> Result<()> {
    use task_scheduler::domain::AgentRunStatus;
    use task_scheduler::store::runs::AgentRunFinishStatus;
    use vw_api_agent_scheduler::{
        AgentSchedulerDispatchBlockerDto as Blocker, AgentSchedulerTaskBoardStatusResponse,
        AgentSchedulerTaskBoardTaskDto, AgentSchedulerTaskPlanPlacementDto as Placement,
        AgentSchedulerWakeResponse,
    };

    let project = tempfile::tempdir()?;
    let project_id = scheduler_project_id(project.path());
    let context = owned_scheduler_context(project.path())?;
    let scope = context.scope().project_scope();
    let database = open_database()?;
    project::store::ProjectStore::new(database.clone())?.create(
        scope.tenant_scope(),
        &project::read_local_test_project(&project_id)?.expect("registered project"),
    )?;

    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let executor = WorkPlanGate { started: started_tx, releases: Arc::new(Mutex::new(release_rx)) };
    let mut gateway = StoreBackedAgentSchedulerGateway::with_executor(executor.clone());
    gateway.store = AgentSchedulerStore::from_database(database);
    let app = router_with_gateway::<()>(Arc::new(gateway.clone())).with_state(());
    let route = format!("/agent-scheduler/projects/{project_id}");
    let tasks = format!("{route}/tasks");
    let commands = format!("{route}/commands");
    let wake_route = format!("{route}/wake");
    let now = || TimestampMs(chrono::Utc::now().timestamp_millis());

    let builtin = task_scheduler::lifecycle_bridge::builtin_task_workflow_snapshot();
    gateway.store.save_task_board_workflow(&context, &builtin)?;

    let mut root_batches =
        vec![("slow".into(), 1, 1), ("fast".into(), 1, 1), ("refill".into(), 1, 1)];
    for (outer, last_inner) in [(1, 5), (2, 4)] {
        let first_inner = if outer == 1 { 2 } else { 1 };
        for inner in first_inner..=last_inner {
            root_batches.push((format!("root-{outer}-{inner}"), outer, inner));
        }
    }
    let parents = [
        ("parent-a", Some(Placement::Root { outer_batch: 1, inner_batch: 1 })),
        ("parent-b", Some(Placement::Root { outer_batch: 1, inner_batch: 1 })),
        ("parent-ordinary", None),
    ];
    let children = [
        ("a-first", "parent-a", 1),
        ("b-first", "parent-b", 1),
        ("a-refill", "parent-a", 1),
        ("b-later", "parent-b", 3),
        ("a-later", "parent-a", 2),
        ("ordinary-first", "parent-ordinary", 1),
        ("ordinary-later", "parent-ordinary", 4),
    ];

    let definitions = std::iter::once(("ordinary", None))
        .chain(root_batches.iter().map(|(id, outer, inner)| {
            (id.as_str(), Some(Placement::Root { outer_batch: *outer, inner_batch: *inner }))
        }))
        .chain(parents.iter().cloned())
        .chain(children.iter().map(|&(id, parent, batch)| {
            (id, Some(Placement::Child { parent_task_id: parent.to_owned(), batch }))
        }));
    for (order, (id, placement)) in definitions.enumerate() {
        let mut request = create_task_request(id);
        request.placement = placement.clone();
        let order = if id.starts_with("parent-") { order + 100 } else { order };
        request.metadata = json!({"prompt": id, "order_no": order});
        let created = post_json(app.clone(), &tasks, &request).await;
        assert_eq!(created["task_id"], id);
        assert_eq!(created["task"]["placement"], serde_json::to_value(placement)?);
    }

    let legacy_steps = json!([{"id": "old", "content": "historical checklist entry"}]);
    let mut legacy = create_task_request("legacy");
    legacy.status = Some(AgentTaskStatusDto::Completed);
    legacy.metadata = json!({"subtasks": legacy_steps});
    post_json(app.clone(), &tasks, &legacy).await;
    let legacy_start = post_json_response(
        app.clone(),
        &commands,
        &json!({"task_id": "legacy", "command": "start"}),
    )
    .await;
    assert_eq!(legacy_start.status(), StatusCode::BAD_REQUEST);

    let move_to = |app: axum::Router, id: &str, status: &str| {
        let commands = commands.clone();
        let request = json!({
            "task_id": id, "command": "move", "payload": {"move_task": {"to_status_key": status}}
        });
        async move {
            let response = post_json(app, &commands, &request).await;
            assert_eq!(response["accepted"], true);
        }
    };
    move_to(app.clone(), "slow", "pending").await;
    let wake: AgentSchedulerWakeResponse = serde_json::from_value(
        post_json(
            app.clone(),
            &wake_route,
            &json!({"dispatch_mode": "auto", "max_concurrent": 2, "now_ms": now()}),
        )
        .await,
    )?;
    assert_eq!(
        wake.dispatched_runs.iter().map(|run| run.task_id.as_str()).collect::<Vec<_>>(),
        ["ordinary", "slow"]
    );

    for (id, _, _) in root_batches.iter().skip(1) {
        move_to(app.clone(), id, "pending").await;
    }
    for (parent, _) in &parents {
        let receipt = post_json(
            app.clone(),
            &format!("{tasks}/{parent}/work-plan"),
            &json!({"plan": {"source": "manual"}, "dispatch_mode": "auto", "max_concurrent": 1}),
        )
        .await;
        let mut expected: Vec<_> =
            children.iter().filter(|child| child.1 == *parent).map(|child| child.0).collect();
        let mut published: Vec<String> = serde_json::from_value(receipt["child_task_ids"].clone())?;
        published.sort();
        expected.sort();
        assert!(published.iter().map(String::as_str).eq(expected));
    }

    let start = |app: axum::Router, id: &str| {
        let commands = commands.clone();
        let request = json!({"task_id": id, "command": "start", "max_concurrent": 3});
        async move { post_json(app, &commands, &request).await }
    };
    let board = |app: axum::Router| {
        let tasks = tasks.clone();
        async move {
            serde_json::from_value::<AgentSchedulerTaskBoardStatusResponse>(
                get_json(app, &tasks).await,
            )
            .expect("project task board")
            .tasks
        }
    };
    fn row<'a>(
        tasks: &'a [AgentSchedulerTaskBoardTaskDto],
        id: &str,
    ) -> &'a AgentSchedulerTaskBoardTaskDto {
        tasks.iter().find(|task| task.task_id.as_ref() == id).expect(id)
    }
    let started = |expected: &str| -> Result<()> {
        let request: LocalAgentExecutionRequest =
            started_rx.recv_timeout(Duration::from_secs(10))?;
        assert_eq!(request.task_id.as_str(), expected);
        if expected == "parent-a" {
            assert!(
                request.prompt.contains("a-later") && request.prompt.contains("work plan result")
            );
        }
        Ok(())
    };

    assert_eq!(start(app.clone(), "fast").await["accepted"], true);
    for id in [
        "fast",
        "refill",
        "a-first",
        "b-first",
        "a-refill",
        "b-later",
        "a-later",
        "ordinary-first",
        "ordinary-later",
        "parent-a",
        "parent-b",
        "parent-ordinary",
    ] {
        started(id)?;
        if id == "b-first" {
            let snapshot = board(app.clone()).await;
            assert!(row(&snapshot, "a-refill").run_id.is_none());
            assert!(matches!(&row(&snapshot, "a-later").read_model.dispatch_blocker,
                Some(Blocker::WaitingForEarlierBatch { task_ids })
                    if task_ids.iter().map(String::as_str).eq(["a-refill"])));
        }
        if id == "b-later" {
            let snapshot = board(app.clone()).await;
            assert_eq!(
                row(&snapshot, "slow").run_id.as_deref(),
                Some(wake.dispatched_runs[1].run_id.as_str())
            );
            assert_eq!(row(&snapshot, "a-refill").status, Some(AgentTaskStatusDto::Completed));
            assert_eq!(row(&snapshot, "b-later").status, Some(AgentTaskStatusDto::Running));
            assert!(row(&snapshot, "a-later").run_id.is_none());
            assert!(matches!(&row(&snapshot, "root-1-2").read_model.dispatch_blocker,
                Some(Blocker::WaitingForEarlierBatch { task_ids }) if task_ids.iter().any(|id| id == "slow")));
        }
        release_tx.send(true)?;
    }

    let slow_id = TaskId::new("slow")?;
    for status in [
        AgentTaskStatus::Dispatched,
        AgentTaskStatus::Acknowledged,
        AgentTaskStatus::Running,
        AgentTaskStatus::Completed,
    ] {
        gateway.store.transition_task_status(&context, &slow_id, status, now())?;
    }
    gateway
        .store
        .finish_run(
            &context,
            &RunId::new(wake.dispatched_runs[1].run_id.clone())?,
            AgentRunFinishStatus::Succeeded,
            now(),
            None,
        )?
        .expect("slow sibling run");

    // The final child/summary callback can already have refilled the slot.
    post_json(app.clone(), &wake_route, &json!({"max_concurrent": 3, "now_ms": now()})).await;
    for id in ["root-1-2", "root-1-3", "root-1-4"] {
        started(id)?;
        release_tx.send(true)?;
    }
    started("root-1-5")?;
    assert_eq!(start(app.clone(), "root-2-1").await["accepted"], true);
    assert!(row(&board(app.clone()).await, "root-2-1").run_id.is_none());

    release_tx.send(false)?;
    let failed =
        wait_for_work_plan_run(&gateway.store, &context, "root-1-5", 1, AgentTaskStatus::Failed)
            .await?;
    assert_eq!(failed.status, AgentRunStatus::Failed);
    let snapshot = board(app.clone()).await;
    assert_eq!(row(&snapshot, "root-1-5").status, Some(AgentTaskStatusDto::Failed));
    assert!(matches!(&row(&snapshot, "root-2-1").read_model.dispatch_blocker,
        Some(Blocker::EarlierBatchFailed { task_ids }) if task_ids.iter().map(String::as_str).eq(["root-1-5"])));

    let mut review_workflow = builtin;
    review_workflow.statuses.push(TaskPoolWorkflowStatusDto {
        key: "review".into(),
        label: "Review result".into(),
        role: TaskPoolWorkflowStatusRoleDto::ReviewReady,
        enabled: true,
        terminal: false,
        next: Some("completed".into()),
        transitions: Vec::new(),
    });
    let running =
        review_workflow.statuses.iter_mut().find(|status| status.key == "running").unwrap();
    running.next = Some("review".into());
    running.transitions.retain(|transition| transition.outcome != "success");
    running.transitions.push(TaskPoolWorkflowTransitionDto {
        outcome: "success".into(),
        label: Some("Submit".into()),
        target: "review".into(),
    });
    gateway.store.save_task_board_workflow(&context, &review_workflow)?;

    drop(app);
    drop(gateway);
    let mut reopened = StoreBackedAgentSchedulerGateway::with_executor(executor);
    reopened.store = AgentSchedulerStore::from_database(open_database()?);
    let resumed_app = router_with_gateway::<()>(Arc::new(reopened.clone())).with_state(());
    let persisted = board(resumed_app.clone()).await;
    assert_eq!(row(&persisted, "root-1-5").run_id.as_deref(), Some(failed.id.as_str()));
    assert_eq!(row(&persisted, "legacy").read_model.legacy_subtasks.as_ref(), Some(&legacy_steps));
    assert!(row(&persisted, "legacy").run_id.is_none());
    assert_eq!(
        row(&persisted, "ordinary-later").placement,
        Some(Placement::Child { parent_task_id: "parent-ordinary".into(), batch: 4 })
    );

    // The release of the physical workspace can lag the committed failure.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let wake = post_json(
                resumed_app.clone(),
                &wake_route,
                &json!({"max_concurrent": 3, "now_ms": now()}),
            )
            .await;
            if wake["dispatched"] == true {
                assert_eq!(wake["task_id"], "root-1-5");
            }
            let run =
                reopened.store.load_latest_run_for_task(&context, &TaskId::new("root-1-5")?)?;
            if run.is_some_and(|run| run.attempt == 2) {
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    started("root-1-5")?;
    release_tx.send(true)?;
    let retry = wait_for_work_plan_run(
        &reopened.store,
        &context,
        "root-1-5",
        2,
        AgentTaskStatus::Submitted,
    )
    .await?;
    assert_eq!(retry.status, AgentRunStatus::Succeeded);
    assert_ne!(retry.id, failed.id);
    let snapshot = board(resumed_app.clone()).await;
    assert_eq!(row(&snapshot, "root-1-5").status_key, "review");
    assert!(matches!(&row(&snapshot, "root-2-1").read_model.dispatch_blocker,
        Some(Blocker::WaitingForEarlierBatch { task_ids }) if task_ids.iter().map(String::as_str).eq(["root-1-5"])));
    assert_eq!(start(resumed_app.clone(), "root-2-1").await["accepted"], true);
    assert!(row(&board(resumed_app.clone()).await, "root-2-1").run_id.is_none());

    move_to(resumed_app.clone(), "root-1-5", "completed").await;
    for id in ["root-2-1", "root-2-2", "root-2-3", "root-2-4"] {
        // Each successful run waits for review before the next batch can start.
        let task_id = TaskId::new(id)?;
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                assert_eq!(start(resumed_app.clone(), id).await["accepted"], true);
                if reopened.store.load_latest_run_for_task(&context, &task_id)?.is_some() {
                    return Ok::<_, anyhow::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await??;
        started(id)?;
        release_tx.send(true)?;
        assert_eq!(
            wait_for_work_plan_run(&reopened.store, &context, id, 1, AgentTaskStatus::Submitted)
                .await?
                .status,
            AgentRunStatus::Succeeded
        );
        move_to(resumed_app.clone(), id, "completed").await;
    }

    let final_board = board(resumed_app.clone()).await;
    for task in final_board.iter().filter(|task| task.task_id.as_ref() != "ordinary") {
        assert_eq!(task.status, Some(AgentTaskStatusDto::Completed), "{}", task.task_id.as_ref());
    }
    assert_eq!(
        reopened
            .store
            .load_run(&context, &RunId::new(wake.dispatched_runs[0].run_id.clone())?)?
            .expect("ordinary run")
            .status,
        AgentRunStatus::Leased
    );

    let events = get_json(resumed_app.clone(), &format!("{route}/events?limit=1000")).await;
    let events = events["events"].as_array().expect("project events");
    for (parent, _) in &parents {
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    event["task_id"] == *parent && event["message"] == "work plan summary queued"
                })
                .count(),
            1
        );
    }
    let attempts = |id: &str, kind: &str| {
        events
            .iter()
            .filter(|event| event["task_id"] == id && event["kind"] == kind)
            .map(|event| event["cursor"].as_i64().expect("event cursor"))
            .collect::<Vec<_>>()
    };
    let starts = attempts("root-1-5", "run_started");
    assert_eq!(starts.len(), 2);
    let retry_queued = events
        .iter()
        .find(|event| {
            event["task_id"] == "root-1-5"
                && event["payload"]["policy_action"] == "retry_failed_task"
        })
        .expect("automatic retry event")["cursor"]
        .as_i64()
        .unwrap();
    assert!(starts[0] < attempts("root-1-5", "run_failed")[0]);
    assert!(retry_queued < starts[1]);
    assert!(starts[1] < attempts("root-1-5", "run_submitted")[0]);
    assert!(attempts("root-1-5", "run_submitted")[0] < attempts("root-2-1", "run_started")[0]);
    for id in ["fast", "refill"]
        .into_iter()
        .chain(children.iter().map(|child| child.0))
        .chain(parents.iter().map(|parent| parent.0))
        .chain(["root-1-2", "root-1-3", "root-1-4", "root-2-1", "root-2-2", "root-2-3", "root-2-4"])
    {
        assert_eq!(attempts(id, "run_started").len(), 1, "{id}");
    }
    for (parent, _) in &parents {
        assert_eq!(attempts(parent, "run_completed").len(), 1);
    }
    let logs = get_json(resumed_app.clone(), &format!("{tasks}/root-1-5/logs")).await;
    for run in [&failed, &retry] {
        assert!(logs["logs"].as_array().expect("task logs").iter().any(|entry| {
            entry["run_id"] == run.id.as_str() && entry["message"] == "local execution started"
        }));
    }
    let artifacts = get_json(resumed_app, &format!("{tasks}/root-1-5/artifacts")).await;
    assert!(artifacts["artifacts"].as_array().expect("task artifacts").iter().any(|artifact| {
        artifact["run_id"] == retry.id.as_str() && artifact["payload"]["success"] == true
    }));
    project::remove_local_test_project(&project_id)?;
    Ok(())
}

async fn wait_for_work_plan_run(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    id: &str,
    attempt: u32,
    task_status: AgentTaskStatus,
) -> Result<task_scheduler::store::runs::StoredAgentRun> {
    let id = TaskId::new(id)?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(run) = store.load_latest_run_for_task(context, &id)?
                && run.attempt == attempt
                && run.finished_at_ms.is_some()
                && store.load_task(context, &id)?.is_some_and(|task| task.status == task_status)
            {
                return Ok(run);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?
}

#[derive(Clone)]
struct WorkPlanGate {
    started: mpsc::Sender<LocalAgentExecutionRequest>,
    releases: Arc<Mutex<mpsc::Receiver<bool>>>,
}

impl LocalAgentExecutor for WorkPlanGate {
    fn lease_workspace(&self, request: LocalAgentWorkspaceRequest) -> Result<LocalAgentWorkspace> {
        Ok(LocalAgentWorkspace {
            path: request.project_path.to_string_lossy().into_owned(),
            slot_id: None,
            branch: None,
        })
    }

    fn execute(&self, request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        self.started.send(request)?;
        anyhow::ensure!(
            self.releases.lock().expect("work plan gate").recv_timeout(Duration::from_secs(10))?,
            "fixture execution failure"
        );
        Ok(LocalAgentExecutionOutput { raw_output: "work plan result".into() })
    }
}
