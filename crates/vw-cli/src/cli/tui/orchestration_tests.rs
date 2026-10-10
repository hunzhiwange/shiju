use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use serde_json::json;
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_gateway_client::vw_api_types::workflow::{WorkflowHumanActionDto, WorkflowPauseDto};
use vw_gateway_client::{CronJobDto, GatewayClient, GatewayEndpoint, GatewayWorkflowStreamEvent};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::{ActiveRunner, V3GatewayApp};
use super::commands::{
    CommandRoute, SCHEDULE_SELECTION_SURFACE, TASK_SELECTION_SURFACE, route_selection,
};
use super::model::UiOverlay;
use super::model::background_task::{CanonicalTaskStatus, TaskControlAction};
use super::runtime::cron::{
    CronJobSnapshot, CronMutation, load_owned_cron_detail_blocking,
    mutate_owned_cron_and_refresh_blocking,
};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime, block_on_gateway};
use super::runtime::orchestration::{
    control_owned_task_blocking, load_orchestration, load_owned_task_detail_blocking,
    load_owned_workflow_detail_blocking,
};
use super::runtime::workflow_run::WorkflowRunOwnership;
use super::state::{TuiAction, TuiState, reduce_tui_state};

fn runtime(port: u16) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("orchestration-session".to_string())),
    )
}

fn scope() -> AgentSchedulerScopeDto {
    AgentSchedulerScopeDto::new(
        ProjectScope::new(TenantScope::default(), ProjectId::from("project-one"))
            .expect("project scope"),
    )
}

fn task_snapshot(
    scope: Option<AgentSchedulerScopeDto>,
) -> super::runtime::orchestration::OrchestrationSnapshot {
    task_snapshot_with_status(scope, CanonicalTaskStatus::Running)
}

fn task_snapshot_with_status(
    scope: Option<AgentSchedulerScopeDto>,
    canonical_status: CanonicalTaskStatus,
) -> super::runtime::orchestration::OrchestrationSnapshot {
    super::runtime::orchestration::OrchestrationSnapshot {
        scope,
        tasks: super::model::background_task::BackgroundTaskPanel {
            tasks: vec![super::model::background_task::BackgroundTask {
                id: "task/owned".to_string(),
                session_id: "project-one".to_string(),
                kind: super::model::background_task::BackgroundTaskKind::Agent,
                title: "Owned task".to_string(),
                status: super::model::background_task::BackgroundTaskStatus::Running,
                canonical_status,
                detail: "running".to_string(),
                log: Vec::new(),
            }],
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn orchestration_panels_render_gateway_snapshots() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut paths = Vec::new();
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().expect("accept");
            let request = read_request(&mut stream);
            let path = request
                .lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .nth(1)
                .unwrap_or_default();
            paths.push(path.to_string());
            let body = match path {
                "/v1/workflow/applications" => json!([{
                    "uuid":"wf-1","name":"Deploy","description":"Ship safely",
                    "created_at_ms":1,"updated_at_ms":2
                }]),
                "/v1/cron" => json!({"jobs":[{
                    "id":"cron-1","name":"Nightly","schedule_kind":"cron",
                    "expression":"0 0 * * *","next_run":"tomorrow","enabled":true
                }]}),
                path if path.ends_with("/tasks") => json!({
                    "project_id":"project-one",
                    "tasks":[{
                        "project_id":"project-one","task_id":"task-1","status_key":"running",
                        "status":"running","priority":1,"order_no":0,"model":"auto",
                        "prompt":"work","retry_count":0,"updated_at_ms":3,
                        "read_model":{"title":"Owned task","status_label":"Running"}
                    }]
                }),
                path if path.ends_with("/workers") => json!({"workers":[{
                    "id":"worker-1","model":"auto","status":"idle","worker_role":"reviewer",
                    "team_id":"team-1","room_id":"room-1","updated_at_ms":4
                }]}),
                _ => json!({}),
            };
            write_json(&mut stream, &body.to_string());
        }
        paths
    });

    let snapshot =
        block_on_gateway(async { Ok(load_orchestration(&runtime(port), Some(scope())).await) })
            .expect("snapshot");
    assert_eq!(snapshot.workflows.items[0].uuid, "wf-1");
    assert_eq!(snapshot.schedules.items[0].id, "cron-1");
    assert_eq!(snapshot.cron_jobs.len(), 1, "canonical Cron DTO must survive in the snapshot");
    assert_eq!(snapshot.agents.members[0].id, "worker-1");
    assert_eq!(snapshot.tasks.tasks[0].id, "task-1");
    assert_eq!(snapshot.tasks.tasks[0].canonical_status, CanonicalTaskStatus::Running);
    assert!(snapshot.warnings.is_empty(), "{:?}", snapshot.warnings);
    assert_eq!(server.join().expect("server").len(), 4);
}

#[test]
fn orchestration_controls_require_owned_stable_ids() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let snapshot = super::runtime::orchestration::OrchestrationSnapshot {
        scope: Some(scope()),
        tasks: super::model::background_task::BackgroundTaskPanel {
            tasks: vec![super::model::background_task::BackgroundTask {
                id: "owned-task".to_string(),
                session_id: "project-one".to_string(),
                kind: super::model::background_task::BackgroundTaskKind::Agent,
                title: "Owned".to_string(),
                status: super::model::background_task::BackgroundTaskStatus::Running,
                canonical_status: CanonicalTaskStatus::Running,
                detail: "running".to_string(),
                log: Vec::new(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let error = control_owned_task_blocking(
        &runtime(port),
        &snapshot,
        "foreign-task",
        TaskControlAction::Cancel,
    )
    .expect_err("foreign ID rejected before request");
    assert!(error.contains("owned by the current project"));

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("owned command");
        let request = read_request(&mut stream);
        write_json(&mut stream, r#"{"accepted":true}"#);
        request
    });
    control_owned_task_blocking(&runtime(port), &snapshot, "owned-task", TaskControlAction::Cancel)
        .expect("owned command");
    let request = server.join().expect("server");
    assert!(request.starts_with("POST /v1/agent-scheduler/projects/project-one/commands "));
    assert!(request.contains(r#""task_id":"owned-task""#));
    assert!(request.contains(r#""command":"cancel""#));
}

#[test]
fn task_control_sends_exact_scheduler_command_dtos() {
    let cases = [
        (TaskControlAction::Start, CanonicalTaskStatus::Draft, "start"),
        (TaskControlAction::Retry, CanonicalTaskStatus::Failed, "retry"),
        (TaskControlAction::Pause, CanonicalTaskStatus::Running, "pause"),
        (TaskControlAction::Resume, CanonicalTaskStatus::Paused, "resume"),
        (TaskControlAction::Cancel, CanonicalTaskStatus::Running, "cancel"),
        (TaskControlAction::Archive, CanonicalTaskStatus::Completed, "archive"),
    ];
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in cases {
            let (mut stream, _) = listener.accept().expect("task control request");
            requests.push(read_request(&mut stream));
            write_json(&mut stream, r#"{"accepted":true}"#);
        }
        requests
    });

    for (action, status, _) in cases {
        let snapshot = task_snapshot_with_status(Some(scope()), status);
        control_owned_task_blocking(&runtime(port), &snapshot, "task/owned", action)
            .expect("allowed task control");
    }

    for (request, (_, _, command)) in server.join().expect("server").into_iter().zip(cases) {
        assert!(request.starts_with("POST /v1/agent-scheduler/projects/project-one/commands "));
        let body: serde_json::Value =
            serde_json::from_str(request_body(&request)).expect("scheduler command JSON");
        assert_eq!(body["task_id"], "task/owned");
        assert_eq!(body["command"], command);
        assert_eq!(body["dispatch_mode"], serde_json::Value::Null);
        assert_eq!(body["payload"], serde_json::Value::Null);
    }
}

#[test]
fn task_control_rejects_illegal_transition_and_gateway_failure_without_local_commit() {
    let zero_listener = TcpListener::bind("127.0.0.1:0").expect("zero-request fixture");
    zero_listener.set_nonblocking(true).expect("nonblocking fixture");
    let zero_port = zero_listener.local_addr().expect("address").port();
    let snapshot = task_snapshot_with_status(Some(scope()), CanonicalTaskStatus::Running);
    let before = snapshot.clone();
    let error = control_owned_task_blocking(
        &runtime(zero_port),
        &snapshot,
        "task/owned",
        TaskControlAction::Retry,
    )
    .expect_err("retry from running must be rejected locally");
    assert!(error.contains("canonical state"));
    assert_eq!(snapshot, before);
    assert!(
        matches!(zero_listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "illegal transition must issue zero Gateway requests"
    );

    let mut foreign_snapshot = before.clone();
    foreign_snapshot.tasks.tasks[0].session_id = "project-two".to_string();
    let error = control_owned_task_blocking(
        &runtime(zero_port),
        &foreign_snapshot,
        "task/owned",
        TaskControlAction::Pause,
    )
    .expect_err("task snapshot from another project must be rejected locally");
    assert!(error.contains("resolved project scope"));
    assert!(
        matches!(zero_listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "foreign project snapshot must issue zero Gateway requests"
    );

    let failure_listener = TcpListener::bind("127.0.0.1:0").expect("failure fixture");
    let failure_port = failure_listener.local_addr().expect("address").port();
    let failure_server = thread::spawn(move || {
        let (mut stream, _) = failure_listener.accept().expect("failed task control request");
        let request = read_request(&mut stream);
        write_error(&mut stream, "task control rejected");
        request
    });
    let error = control_owned_task_blocking(
        &runtime(failure_port),
        &snapshot,
        "task/owned",
        TaskControlAction::Pause,
    )
    .expect_err("Gateway failure must be surfaced");
    assert!(error.contains("task control rejected"));
    assert_eq!(snapshot, before, "Gateway failure must preserve the local snapshot");
    assert!(failure_server.join().expect("failure server").contains(r#""command":"pause""#));
}

#[test]
fn task_control_refreshes_snapshot_only_after_gateway_success() {
    let success_listener = TcpListener::bind("127.0.0.1:0").expect("success fixture");
    let success_port = success_listener.local_addr().expect("address").port();
    let success_server = thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..5 {
            let (mut stream, _) = success_listener.accept().expect("control or refresh request");
            let request = read_request(&mut stream);
            let request_line = request.lines().next().unwrap_or_default().to_string();
            let path = request_line.split_whitespace().nth(1).unwrap_or_default();
            let body = if request_line.starts_with("POST ") {
                json!({"accepted": true})
            } else if path == "/v1/workflow/applications" {
                json!([])
            } else if path == "/v1/cron" {
                json!({"jobs": []})
            } else if path.ends_with("/tasks") {
                json!({
                    "project_id": "project-one",
                    "tasks": [{
                        "project_id": "project-one",
                        "task_id": "task/owned",
                        "status_key": "paused",
                        "status": "paused",
                        "priority": 1,
                        "order_no": 0,
                        "model": "auto",
                        "prompt": "work",
                        "retry_count": 0,
                        "updated_at_ms": 4,
                        "read_model": {"title": "Owned task", "status_label": "Paused"}
                    }]
                })
            } else if path.ends_with("/workers") {
                json!({"workers": []})
            } else {
                panic!("unexpected refresh path: {path}");
            };
            write_json(&mut stream, &body.to_string());
            requests.push(request_line);
        }
        requests
    });
    let mut success_state = TuiState::default();
    success_state.orchestration =
        task_snapshot_with_status(Some(scope()), CanonicalTaskStatus::Running);
    let mut success_app = V3GatewayApp::new(runtime(success_port), success_state);
    let mut success_runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));
    confirm_task_control(&mut success_runner, &mut success_app, "/tasks pause task/owned");
    assert_eq!(
        success_app.state().orchestration.tasks.tasks[0].canonical_status,
        CanonicalTaskStatus::Paused,
        "Gateway success must refresh and commit the canonical snapshot"
    );
    let success_requests = success_server.join().expect("success server");
    assert_eq!(success_requests.len(), 5);
    assert!(
        success_requests[0].starts_with("POST /v1/agent-scheduler/projects/project-one/commands ")
    );

    let failure_listener = TcpListener::bind("127.0.0.1:0").expect("failure fixture");
    let failure_port = failure_listener.local_addr().expect("address").port();
    let failure_server = thread::spawn(move || {
        let (mut stream, _) = failure_listener.accept().expect("failed control request");
        let request = read_request(&mut stream);
        write_error(&mut stream, "task control rejected");
        thread::sleep(Duration::from_millis(100));
        failure_listener.set_nonblocking(true).expect("nonblocking failure fixture");
        (request, failure_listener.accept().is_ok())
    });
    let mut failure_state = TuiState::default();
    failure_state.orchestration =
        task_snapshot_with_status(Some(scope()), CanonicalTaskStatus::Running);
    let mut failure_app = V3GatewayApp::new(runtime(failure_port), failure_state);
    let before_failure = failure_app.state().orchestration.clone();
    let mut failure_runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));
    confirm_task_control(&mut failure_runner, &mut failure_app, "/tasks pause task/owned");
    assert_eq!(failure_app.state().orchestration, before_failure);
    let (failed_request, refresh_attempted) = failure_server.join().expect("failure server");
    assert!(failed_request.contains(r#""command":"pause""#));
    assert!(!refresh_attempted, "failed command must not refresh Scheduler snapshot");
}

fn confirm_task_control(runner: &mut ActiveRunner, app: &mut V3GatewayApp, command: &str) {
    runner.execute_action(
        app,
        V3Action::Submit { text: command.to_string(), attachments: Vec::new() },
    );
    let UiOverlay::Confirm(confirmation) =
        app.state().overlays.active().expect("task control confirmation")
    else {
        panic!("expected typed task confirmation");
    };
    let request_id = confirmation.request_id.clone();
    runner.execute_action(
        app,
        V3Action::ResolveApproval { request_id, decision: V3ApprovalDecision::Allow },
    );
}

#[test]
fn task_detail_requires_current_project_snapshot() {
    assert_eq!(
        route_selection(TASK_SELECTION_SURFACE, " task/owned "),
        Ok(CommandRoute::OpenTaskDetail { task_id: "task/owned".to_string() })
    );
    assert!(route_selection(TASK_SELECTION_SURFACE, " ").is_err());

    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    listener.set_nonblocking(true).expect("nonblocking fixture");
    let port = listener.local_addr().expect("address").port();
    let snapshot = task_snapshot(Some(scope()));

    let error = load_owned_task_detail_blocking(&runtime(port), &snapshot, "task/foreign")
        .expect_err("foreign task ID must be rejected before a request");
    assert!(error.contains("current project snapshot"));

    let missing_scope = task_snapshot(None);
    let error = load_owned_task_detail_blocking(&runtime(port), &missing_scope, "task/owned")
        .expect_err("owned task without canonical scope must be rejected before a request");
    assert!(error.contains("resolved project scope"));
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "foreign task IDs and unresolved scope must issue zero Gateway requests"
    );
}

#[test]
fn task_detail_isolates_graph_log_and_artifact_failures() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut paths = Vec::new();
        for request_index in 0..12 {
            let (mut stream, _) = listener.accept().expect("task detail request");
            let request = read_request(&mut stream);
            let path = request
                .lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            let round = request_index / 3;
            let source_fails = (round == 1 && path.ends_with("/task-graph"))
                || (round == 2 && path.ends_with("/logs"))
                || (round == 3 && path.ends_with("/artifacts"));
            paths.push(path.clone());
            if source_fails {
                write_error(&mut stream, "source unavailable token=source-secret");
            } else if path.ends_with("/task-graph") {
                write_json(
                    &mut stream,
                    &json!({
                        "project_id": "project-one",
                        "edges": [{
                            "predecessor_task_id": "task/dependency",
                            "successor_task_id": "task/owned"
                        }],
                        "tasks": [{
                            "task_id": "task/owned",
                            "status": "running",
                            "readiness": "waiting_dependencies",
                            "waiting_task_ids": (0..20)
                                .map(|index| format!("task/waiting-{index}"))
                                .collect::<Vec<_>>(),
                            "blocking_task_ids": ["task/blocked"]
                        }]
                    })
                    .to_string(),
                );
            } else if path.ends_with("/logs") {
                let logs = (0..25)
                    .map(|index| {
                        json!({
                            "task_id": "task/owned",
                            "seq_no": index,
                            "timestamp_ms": index,
                            "from_status_key": "queued",
                            "status_from": "queued",
                            "to_status_key": "running",
                            "status_to": "running",
                            "message": format!(
                                "log-{index} token=log-secret {}",
                                "x".repeat(900)
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                write_json(
                    &mut stream,
                    &json!({"task_id": "task/owned", "logs": logs}).to_string(),
                );
            } else if path.ends_with("/artifacts") {
                let artifacts = (0..25)
                    .map(|index| {
                        json!({
                            "task_id": "task/owned",
                            "run_id": format!("run-{index}"),
                            "kind": "report",
                            "name": format!("artifact-{index}"),
                            "path": format!("/private/output-{index}.txt"),
                            "payload": {"content": "payload-secret"},
                            "created_at_ms": index
                        })
                    })
                    .collect::<Vec<_>>();
                write_json(
                    &mut stream,
                    &json!({"task_id": "task/owned", "artifacts": artifacts}).to_string(),
                );
            } else {
                panic!("unexpected task detail path: {path}");
            }
        }
        paths
    });
    let snapshot = task_snapshot(Some(scope()));
    let runtime = runtime(port);

    let success = load_owned_task_detail_blocking(&runtime, &snapshot, "task/owned")
        .expect("all task detail sources");
    let rendered = success.render_text();
    assert!(rendered.contains("Scheduler 任务详情"));
    assert!(rendered.contains("WaitingDependencies"));
    assert!(rendered.contains("task/waiting-11"));
    assert!(!rendered.contains("task/waiting-12"), "dependency count must be bounded");
    assert!(rendered.contains("log-24"), "recent bounded logs must be retained");
    assert!(!rendered.contains("log-4"), "logs beyond the fixed limit must be omitted");
    assert!(rendered.contains("artifact-19"));
    assert!(!rendered.contains("artifact-20"), "artifact count must be bounded");
    assert!(!rendered.contains("log-secret"));
    assert!(!rendered.contains("payload-secret"), "artifact payload must never be rendered");
    assert!(rendered.chars().count() < 20_000, "task detail transcript must be bounded");

    for (failed_source, preserved) in [
        ("任务图：加载失败", ["log-24", "artifact-0"]),
        ("日志：加载失败", ["WaitingDependencies", "artifact-0"]),
        ("产物：加载失败", ["WaitingDependencies", "log-24"]),
    ] {
        let detail = load_owned_task_detail_blocking(&runtime, &snapshot, "task/owned")
            .expect("one failed source must still return the other task details");
        let rendered = detail.render_text();
        assert!(rendered.contains(failed_source));
        assert!(preserved.into_iter().all(|text| rendered.contains(text)));
        assert!(!rendered.contains("source-secret"));
    }

    let paths = server.join().expect("server");
    for expected in [
        "/v1/agent-scheduler/projects/project-one/task-graph",
        "/v1/agent-scheduler/projects/project-one/tasks/task%2Fowned/logs",
        "/v1/agent-scheduler/projects/project-one/tasks/task%2Fowned/artifacts",
    ] {
        assert_eq!(paths.iter().filter(|path| path.as_str() == expected).count(), 4);
    }
}

#[test]
fn workflow_detail_renders_safe_summary_and_isolates_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in [
            Some(json!({
                "uuid": "wf/current",
                "name": "Deploy",
                "description": "Ship safely",
                "workflow_yaml": "workflow:\n  graph:\n    nodes:\n      - id: start\n        data:\n          api_key: super-secret\n      - id: finish\n    edges:\n      - id: start-finish\n        source: start\n        target: finish\ncredentials:\n  token: hidden-token\n",
                "created_at_ms": 1,
                "updated_at_ms": 42
            })),
            None,
        ] {
            let (mut stream, _) = listener.accept().expect("accept");
            requests.push(read_request(&mut stream));
            match response {
                Some(body) => write_json(&mut stream, &body.to_string()),
                None => write_error(&mut stream, "workflow detail unavailable"),
            }
        }
        requests
    });
    let snapshot = super::runtime::orchestration::OrchestrationSnapshot {
        workflows: super::model::workflow_panel::WorkflowPanel {
            items: vec![
                super::model::workflow_panel::WorkflowPanelItem {
                    uuid: "wf/current".to_string(),
                    name: "Deploy".to_string(),
                    description: "Ship safely".to_string(),
                },
                super::model::workflow_panel::WorkflowPanelItem {
                    uuid: "wf-fail".to_string(),
                    name: "Failing".to_string(),
                    description: "Still owned".to_string(),
                },
            ],
            ..Default::default()
        },
        schedules: super::model::schedule_panel::SchedulePanel {
            error: Some("existing schedule warning".to_string()),
            ..Default::default()
        },
        warnings: vec!["existing orchestration warning".to_string()],
        ..Default::default()
    };

    let detail = load_owned_workflow_detail_blocking(&runtime(port), &snapshot, "wf/current")
        .expect("owned workflow detail");
    let rendered = detail.render_text();
    assert!(rendered.contains("Deploy"));
    assert!(rendered.contains("Ship safely"));
    assert!(rendered.contains("42"));
    assert!(rendered.contains("2 个节点"));
    assert!(rendered.contains("1 条连接"));
    for sensitive in ["super-secret", "hidden-token", "api_key", "credentials"] {
        assert!(!rendered.contains(sensitive), "leaked sensitive Workflow field: {sensitive}");
    }

    let before_failure = snapshot.clone();
    let error = load_owned_workflow_detail_blocking(&runtime(port), &snapshot, "wf-fail")
        .expect_err("Gateway failure should remain visible");
    assert!(error.contains("workflow detail unavailable"));
    assert_eq!(snapshot, before_failure, "detail failure must not replace other panels");

    let requests = server.join().expect("server");
    assert!(requests[0].starts_with("GET /v1/workflow/applications/wf%2Fcurrent "));
    assert!(requests[1].starts_with("GET /v1/workflow/applications/wf-fail "));
    let unknown = load_owned_workflow_detail_blocking(&runtime(port), &snapshot, "stale-workflow")
        .expect_err("stale UUID must be rejected before a request");
    assert!(unknown.contains("current Workflow snapshot"));
    assert_eq!(requests.len(), 2, "stale UUID must add zero Gateway requests");
}

#[test]
fn schedule_selection_requires_snapshot_job() {
    assert_eq!(
        route_selection(SCHEDULE_SELECTION_SURFACE, " cron/owned "),
        Ok(CommandRoute::OpenCronDetail { job_id: "cron/owned".to_string() })
    );
    assert!(route_selection(SCHEDULE_SELECTION_SURFACE, " ").is_err());

    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    listener.set_nonblocking(true).expect("nonblocking fixture");
    let port = listener.local_addr().expect("address").port();
    let jobs = vec![CronJobSnapshot::from_dto(&cron_job("cron/owned"))];
    let error = load_owned_cron_detail_blocking(&runtime(port), &jobs, "cron/stale")
        .expect_err("stale Cron ID must be rejected before a request");
    assert!(error.contains("current Cron snapshot"));
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "unknown ID must issue zero Gateway requests"
    );
}

#[test]
fn cron_detail_renders_bounded_run_history() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fixture");
    let port = listener.local_addr().expect("address").port();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();

        let (mut success, _) = listener.accept().expect("success history");
        requests.push(read_request(&mut success));
        let runs = (0..25)
            .map(|index| {
                json!({
                    "id": index,
                    "job_id": "cron/owned",
                    "started_at": format!("start-{index}"),
                    "finished_at": format!("finish-{index}"),
                    "status": "ok",
                    "output": format!("run-output-{index} token=run-secret {}", "x".repeat(900)),
                    "duration_ms": index * 10
                })
            })
            .collect::<Vec<_>>();
        write_json(&mut success, &json!({"runs": runs}).to_string());

        let (mut empty, _) = listener.accept().expect("empty history");
        requests.push(read_request(&mut empty));
        write_json(&mut empty, r#"{"runs":[]}"#);

        for _ in 0..2 {
            let (mut unsupported, _) = listener.accept().expect("unsupported history");
            requests.push(read_request(&mut unsupported));
            write_status_json(&mut unsupported, "404 Not Found", r#"{"error":"missing"}"#);
        }
        requests
    });
    let jobs = ["cron/owned", "cron-empty", "cron-unsupported"]
        .into_iter()
        .map(cron_job)
        .map(|job| CronJobSnapshot::from_dto(&job))
        .collect::<Vec<_>>();
    let before = jobs.clone();

    let detail = load_owned_cron_detail_blocking(&runtime(port), &jobs, "cron/owned")
        .expect("owned Cron detail");
    let rendered = detail.render_text();
    assert!(rendered.contains("Nightly cron/owned"));
    assert!(rendered.contains("0 0 * * *"));
    assert!(rendered.contains("run-output-0"));
    assert!(!rendered.contains("run-output-24"), "history count must be bounded");
    assert!(!rendered.contains("command-secret"));
    assert!(!rendered.contains("run-secret"));
    assert!(rendered.contains("[redacted]"));
    assert!(rendered.chars().count() < 15_000, "Cron transcript must be bounded");

    let empty = load_owned_cron_detail_blocking(&runtime(port), &jobs, "cron-empty")
        .expect("empty history remains a successful detail");
    assert!(empty.render_text().contains("暂无运行记录"));

    let error = load_owned_cron_detail_blocking(&runtime(port), &jobs, "cron-unsupported")
        .expect_err("legacy Gateway must report unsupported history");
    assert!(error.contains("不支持定时任务历史接口"));
    assert_eq!(jobs, before, "Cron failure must not replace other orchestration sources");

    let requests = server.join().expect("server");
    let paths = requests
        .iter()
        .map(|request| {
            request.lines().next().unwrap_or_default().split_whitespace().nth(1).unwrap_or_default()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "/v1/cron/cron%2Fowned/runs",
            "/v1/cron/cron-empty/runs",
            "/v1/cron/cron-unsupported/runs",
            "/v1/cron/runs/cron-unsupported",
        ]
    );
}

#[test]
fn cron_mutation_refreshes_after_gateway_success_only() {
    let original_job = cron_job("cron/owned");
    let original_snapshot = vec![CronJobSnapshot::from_dto(&original_job)];

    let unknown_listener = TcpListener::bind("127.0.0.1:0").expect("unknown fixture");
    unknown_listener.set_nonblocking(true).expect("nonblocking unknown fixture");
    let unknown_port = unknown_listener.local_addr().expect("unknown address").port();
    let error = mutate_owned_cron_and_refresh_blocking(
        &runtime(unknown_port),
        &original_snapshot,
        "cron/stale",
        CronMutation::SetEnabled(false),
    )
    .expect_err("stale Cron ID must fail before mutation");
    assert!(error.contains("current Cron snapshot"));
    assert!(
        matches!(unknown_listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "stale Cron ID must issue zero Gateway requests"
    );

    let update_listener = TcpListener::bind("127.0.0.1:0").expect("update fixture");
    let update_port = update_listener.local_addr().expect("update address").port();
    let update_server = thread::spawn(move || {
        let mut requests = Vec::new();
        let (mut update, _) = update_listener.accept().expect("update request");
        requests.push(read_request(&mut update));
        write_json(&mut update, r#"{"status":"ok","job":null}"#);

        let (mut refresh, _) = update_listener.accept().expect("refresh request");
        requests.push(read_request(&mut refresh));
        write_json(
            &mut refresh,
            &json!({"jobs": [{
                "id":"cron/owned","name":"Nightly","schedule_kind":"cron",
                "expression":"0 0 * * *","next_run":"later","enabled":false
            }]})
            .to_string(),
        );
        requests
    });
    let refreshed = mutate_owned_cron_and_refresh_blocking(
        &runtime(update_port),
        &original_snapshot,
        "cron/owned",
        CronMutation::SetEnabled(false),
    )
    .expect("successful update must refresh Cron source");
    assert_eq!(refreshed.schedules.items.len(), 1);
    assert!(!refreshed.schedules.items[0].enabled);
    assert_eq!(refreshed.cron_jobs.len(), 1);
    let update_requests = update_server.join().expect("update server");
    assert!(update_requests[0].starts_with("PATCH /v1/cron/cron%2Fowned "));
    assert_eq!(request_body(&update_requests[0]), r#"{"enabled":false}"#);
    assert!(update_requests[1].starts_with("GET /v1/cron "));

    let delete_listener = TcpListener::bind("127.0.0.1:0").expect("delete fixture");
    let delete_port = delete_listener.local_addr().expect("delete address").port();
    let delete_server = thread::spawn(move || {
        let mut requests = Vec::new();
        let (mut delete, _) = delete_listener.accept().expect("delete request");
        requests.push(read_request(&mut delete));
        write_json(&mut delete, "null");
        let (mut refresh, _) = delete_listener.accept().expect("delete refresh");
        requests.push(read_request(&mut refresh));
        write_json(&mut refresh, r#"{"jobs":[]}"#);
        requests
    });
    let refreshed = mutate_owned_cron_and_refresh_blocking(
        &runtime(delete_port),
        &original_snapshot,
        "cron/owned",
        CronMutation::Delete,
    )
    .expect("successful delete must refresh Cron source");
    assert!(refreshed.schedules.items.is_empty());
    assert!(refreshed.cron_jobs.is_empty());
    let delete_requests = delete_server.join().expect("delete server");
    assert!(delete_requests[0].starts_with("DELETE /v1/cron/cron%2Fowned "));
    assert!(delete_requests[1].starts_with("GET /v1/cron "));

    let failure_listener = TcpListener::bind("127.0.0.1:0").expect("failure fixture");
    let failure_port = failure_listener.local_addr().expect("failure address").port();
    let failure_server = thread::spawn(move || {
        let (mut mutation, _) = failure_listener.accept().expect("failed mutation");
        let request = read_request(&mut mutation);
        write_error(&mut mutation, "mutation rejected");
        thread::sleep(Duration::from_millis(100));
        failure_listener.set_nonblocking(true).expect("nonblocking failure fixture");
        let refresh_attempted = failure_listener.accept().is_ok();
        (request, refresh_attempted)
    });
    let before_failure = original_snapshot.clone();
    let error = mutate_owned_cron_and_refresh_blocking(
        &runtime(failure_port),
        &original_snapshot,
        "cron/owned",
        CronMutation::SetEnabled(true),
    )
    .expect_err("Gateway mutation failure must not refresh Cron source");
    assert!(error.contains("mutation rejected"));
    assert_eq!(original_snapshot, before_failure, "failure must preserve the local snapshot");
    let (failed_request, refresh_attempted) = failure_server.join().expect("failure server");
    assert!(failed_request.starts_with("PATCH /v1/cron/cron%2Fowned "));
    assert!(!refresh_attempted, "failed mutation must not issue a refresh request");
}

#[test]
fn workflow_pause_projects_typed_questions_without_persisting_token() {
    let pause = WorkflowPauseDto {
        node_id: "approval".to_string(),
        title: "Release review".to_string(),
        form_token: "secret-workflow-token".to_string(),
        form: json!({
            "fields": [
                {
                    "name": "comment",
                    "label": "Review comment",
                    "type": "text",
                    "required": true
                },
                {
                    "name": "risk",
                    "label": "Risk",
                    "type": "select",
                    "options": [
                        {"label": "Low", "value": "low"},
                        {"label": "High", "value": "high"}
                    ]
                }
            ]
        }),
        actions: vec![
            WorkflowHumanActionDto { id: "approve".to_string(), label: "Approve".to_string() },
            WorkflowHumanActionDto { id: "reject".to_string(), label: "Reject".to_string() },
        ],
    };
    let mut ownership = WorkflowRunOwnership::new_for_session(
        "workflow-run-1".to_string(),
        "wf-owned".to_string(),
        "session-owned".to_string(),
    );
    ownership.project_event(GatewayWorkflowStreamEvent::WorkflowFinished {
        task_id: "task-1".to_string(),
        workflow_run_id: "run-1".to_string(),
        data: json!({"status": "paused"}),
        pause: Some(pause),
    });

    let question =
        ownership.pause_question().expect("valid Workflow form").expect("paused Workflow question");
    assert_eq!(question.session_id, "session-owned");
    assert_eq!(question.prompts.len(), 3);
    assert_eq!(question.prompts[0].header, "Review comment");
    assert!(question.prompts[0].allow_custom_input);
    assert_eq!(question.prompts[1].options[0].label, "Low");
    assert_eq!(question.prompts[2].options[0].label, "Approve [approve]");
    assert!(question.request_id.starts_with("workflow-pause-"));
    assert!(!question.request_id.contains("secret-workflow-token"));

    let mut state = TuiState::default();
    state.session.session_id = Some("session-owned".to_string());
    reduce_tui_state(&mut state, TuiAction::WorkflowPauseQuestionReplaced(Some(question)));
    let persisted = serde_json::to_string(&state.to_chat_session()).expect("serialize snapshot");
    assert!(!persisted.contains("secret-workflow-token"));
    assert!(!format!("{state:?}").contains("secret-workflow-token"));
    assert!(!format!("{ownership:?}").contains("secret-workflow-token"));
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    loop {
        let read = stream.read(&mut buffer).expect("read");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..header_end + 4]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(str::trim)
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap_or_default();
            if bytes.len() >= header_end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("utf8")
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

fn write_json(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write");
}

fn write_error(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write");
}

fn write_status_json(stream: &mut std::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write");
}

fn cron_job(id: &str) -> CronJobDto {
    CronJobDto {
        id: id.to_string(),
        name: Some(format!("Nightly {id}")),
        job_type: "command".to_string(),
        schedule_kind: "cron".to_string(),
        expression: "0 0 * * *".to_string(),
        at: None,
        every_ms: None,
        command: "deploy token=command-secret".to_string(),
        prompt: Some("Review the nightly deployment".to_string()),
        model: Some("auto".to_string()),
        agent: Some("reviewer".to_string()),
        acp_agent: Some("codex".to_string()),
        project_path: Some("/workspace".to_string()),
        wake: true,
        fallbacks: vec!["backup".to_string()],
        full_access: false,
        task_pool: true,
        delivery_mode: "ui".to_string(),
        delivery_channel: Some("terminal".to_string()),
        delivery_to: Some("owner".to_string()),
        delivery_best_effort: true,
        delete_after_run: false,
        next_run: "tomorrow".to_string(),
        last_run: Some("yesterday".to_string()),
        last_status: Some("ok".to_string()),
        last_output: Some("completed".to_string()),
        enabled: true,
    }
}
