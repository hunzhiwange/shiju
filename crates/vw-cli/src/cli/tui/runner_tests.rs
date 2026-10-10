use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::id::ProjectId;
use vw_gateway_client::{CronJobDto, GatewayClient, GatewayEndpoint};
use vw_tui::V3Action;

use super::app::{
    ActiveRunner, STREAM_EVENT_BUDGET, TERMINAL_EVENT_BUDGET, V3GatewayApp,
    apply_ready_terminal_events,
};
use super::commands::{
    SCHEDULE_SELECTION_SURFACE, TASK_SELECTION_SURFACE, WORKSPACE_FILES_SURFACE,
};
use super::model::background_task::{
    BackgroundTask, BackgroundTaskKind, BackgroundTaskPanel, BackgroundTaskStatus,
    CanonicalTaskStatus,
};
use super::model::schedule_panel::{SchedulePanel, SchedulePanelItem};
use super::model::workflow_panel::{WorkflowPanel, WorkflowPanelItem};
use super::runtime::cron::CronJobSnapshot;
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

#[test]
fn active_runner_drain_yields_after_stream_budget() {
    let mut app = test_app(GatewayEndpoint::new("127.0.0.1", 9));
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.install_test_stream(
        (0..=STREAM_EVENT_BUDGET)
            .map(|_| vw_gateway_client::GatewayChatStreamEvent::Delta("x".to_string())),
    );

    assert!(runner.drain(&mut app), "a full stream budget must yield remaining work");
    let rendered = app.transcript_lines(80, true).iter().map(ToString::to_string).collect::<String>();
    assert_eq!(rendered.matches('x').count(), STREAM_EVENT_BUDGET);

    runner.drain(&mut app);
    let rendered = app.transcript_lines(80, true).iter().map(ToString::to_string).collect::<String>();
    assert_eq!(rendered.matches('x').count(), STREAM_EVENT_BUDGET + 1);
}

#[test]
fn ready_key_burst_is_applied_before_next_frame() {
    let mut app = test_app(GatewayEndpoint::new("127.0.0.1", 9));
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    let mut events = (0..TERMINAL_EVENT_BUDGET)
        .map(|_| Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)))
        .chain(std::iter::once(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        ))));
    let first = events.next().expect("non-empty input burst");

    let outcome = apply_ready_terminal_events(&mut app, &mut runner, first, || Ok(events.next()))
        .expect("apply ready terminal events");

    assert_eq!(outcome.processed, TERMINAL_EVENT_BUDGET);
    assert_eq!(app.widget().draft(), "a".repeat(TERMINAL_EVENT_BUDGET));
}

#[test]
fn active_runner_executes_local_actions_and_drains_queue() {
    let (endpoint, requests, server) = two_turn_gateway_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|draft| {
        Ok(Some(format!("{draft} edited").trim().to_string()))
    }));

    app.widget_mut().replace_draft("draft");
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    drain_until(&mut runner, &mut app, |runner, _| !runner.editor_in_flight());
    assert_eq!(app.widget().draft(), "draft edited");

    runner.execute_action(&mut app, V3Action::ShowTranscript);
    assert!(runner.has_transcript_pager());

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "first".to_string(), attachments: Vec::new() },
    );
    runner.execute_action(
        &mut app,
        V3Action::Queue { text: "second".to_string(), attachments: Vec::new() },
    );
    assert_eq!(app.widget().queued_input_count(), 1);

    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.has_active_stream() && app.widget().queued_input_count() == 0
    });

    let paths = requests.try_iter().collect::<Vec<_>>();
    assert_eq!(
        paths
            .iter()
            .filter(|path| path.as_str() == "/v1/chat/stream?directory=%2Fworkspace")
            .count(),
        2,
        "the queued prompt must be submitted exactly once after the first terminal event"
    );
    server.join().expect("two-turn gateway fixture");
}

#[test]
fn interrupt_stops_the_matching_resumable_stream() {
    let (endpoint, requests, server) = interrupt_gateway_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "long running".to_string(), attachments: Vec::new() },
    );
    drain_until(&mut runner, &mut app, |_, app| {
        app.transcript_lines(80, true).iter().any(|line| line.to_string().contains("working"))
    });
    let request_id =
        runner.active_request_id().expect("active stream has a stable request id").to_string();
    runner.execute_action(&mut app, V3Action::Interrupt);

    drain_until(&mut runner, &mut app, |runner, _| !runner.has_active_stream());
    let recorded = requests.recv_timeout(Duration::from_secs(2)).expect("record stream and stop");
    assert_eq!(recorded.stream_request_id, request_id);
    assert_eq!(recorded.stop_path, format!("/v1/chat/stream/{request_id}/stop"));
    server.join().expect("interrupt gateway fixture");
}

#[test]
fn file_search_discards_stale_results_and_applies_latest_query() {
    let (endpoint, requests, server) = out_of_order_file_search_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    app.widget_mut().replace_draft("@old");
    runner.execute_action(
        &mut app,
        V3Action::FileSearchQueryChanged { query: Some("old".to_string()) },
    );
    app.widget_mut().replace_draft("@new");
    runner.execute_action(
        &mut app,
        V3Action::FileSearchQueryChanged { query: Some("new".to_string()) },
    );

    let recorded = requests.recv_timeout(Duration::from_secs(3)).expect("record file searches");
    assert_eq!(recorded.len(), 2);
    assert!(recorded.iter().any(|path| {
        path == "/v1/find/file?directory=%2Fworkspace&query=old&dirs=false&type=file&limit=20"
    }));
    assert!(recorded.iter().any(|path| {
        path == "/v1/find/file?directory=%2Fworkspace&query=new&dirs=false&type=file&limit=20"
    }));
    server.join().expect("out-of-order file search fixture");

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        runner.drain(&mut app);
        let rendered = render_widget(&app);
        if rendered.contains("src/new.rs") {
            thread::sleep(Duration::from_millis(50));
            runner.drain(&mut app);
            let settled = render_widget(&app);
            assert!(settled.contains("src/new.rs"));
            assert!(!settled.contains("src/old.rs"));
            break;
        }
        assert!(Instant::now() < deadline, "latest file search result was not rendered");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn file_search_error_ends_loading_without_changing_the_draft() {
    let (endpoint, request, server) = failing_file_search_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    app.widget_mut().replace_draft("@broken");
    runner.execute_action(
        &mut app,
        V3Action::FileSearchQueryChanged { query: Some("broken".to_string()) },
    );

    assert_eq!(
        request.recv_timeout(Duration::from_secs(3)).expect("record failed file search"),
        "/v1/find/file?directory=%2Fworkspace&query=broken&dirs=false&type=file&limit=20"
    );
    server.join().expect("failing file search fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("file search unavailable")
    });
    assert_eq!(app.widget().draft(), "@broken");
}

#[test]
fn runner_renders_gateway_session_diff() {
    let (endpoint, request, server) = session_diff_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/diff".to_string(), attachments: Vec::new() },
    );

    assert!(!runner.has_active_stream(), "/diff must not start a chat stream");
    assert_eq!(
        request.recv_timeout(Duration::from_secs(3)).expect("record session diff request"),
        "/v1/session/runner-session/diff?directory=%2Fworkspace"
    );
    server.join().expect("session diff gateway fixture");

    drain_until(&mut runner, &mut app, |_, app| {
        let rendered = render_widget(app);
        rendered.contains("src/lib.rs") && rendered.contains("new value")
    });
}

#[test]
fn runner_reports_empty_and_failed_session_diff() {
    let (endpoint, request, server) = session_diff_response_fixture("200 OK", "[]", Duration::ZERO);
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/diff".to_string(), attachments: Vec::new() },
    );
    request.recv_timeout(Duration::from_secs(3)).expect("record empty diff request");
    server.join().expect("empty diff gateway fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        transcript_text(app).contains("当前会话没有文件变化")
    });

    let (endpoint, request, server) = session_diff_response_fixture(
        "503 Service Unavailable",
        r#"{"message":"diff unavailable"}"#,
        Duration::ZERO,
    );
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/diff".to_string(), attachments: Vec::new() },
    );
    request.recv_timeout(Duration::from_secs(3)).expect("record failed diff request");
    server.join().expect("failed diff gateway fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        transcript_text(app).contains("failed to load session diff")
    });
}

#[test]
fn runner_reports_missing_session_id_for_diff() {
    let client =
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).expect("test gateway client");
    let runtime =
        GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")));
    let mut state = TuiState::default();
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    let mut app = V3GatewayApp::new(runtime, state);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Select { surface: "command_palette".to_string(), item_id: "diff".to_string() },
    );

    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("gateway runtime session id is required")
    });
}

#[test]
fn runner_renders_latest_doctor_result() {
    let (endpoint, requests, server) = out_of_order_doctor_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/doctor".to_string(), attachments: Vec::new() },
    );
    assert!(!runner.has_active_stream(), "/doctor must not start a chat stream");
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(3)).expect("record first doctor request"),
        "/v1/doctor"
    );

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/doctor".to_string(), attachments: Vec::new() },
    );
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(3)).expect("record second doctor request"),
        "/v1/doctor"
    );
    server.join().expect("out-of-order doctor fixture");

    drain_until(&mut runner, &mut app, |_, app| render_widget(app).contains("latest result"));
    thread::sleep(Duration::from_millis(80));
    runner.drain(&mut app);
    let rendered = render_widget(&app);
    assert!(rendered.contains("latest result"));
    assert!(!rendered.contains("stale result"));
}

#[test]
fn runner_renders_sanitized_doctor_error() {
    let (endpoint, request, server) = doctor_error_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/doctor".to_string(), attachments: Vec::new() },
    );
    assert_eq!(
        request.recv_timeout(Duration::from_secs(3)).expect("record failed doctor request"),
        "/v1/doctor"
    );
    server.join().expect("failed doctor fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        render_widget(app).contains("failed to load Gateway diagnostics")
    });
    let rendered = render_widget(&app);
    assert!(!rendered.contains("token=super-secret"));
    assert!(!rendered.contains("Bearer super-secret"));
}

#[test]
fn runner_projects_workflow_stream_and_single_terminal() {
    let (endpoint, requests, server) = workflow_gateway_fixture();
    let mut app = test_app(endpoint);
    app.state_mut().orchestration.workflows = WorkflowPanel {
        items: vec![
            WorkflowPanelItem {
                uuid: "wf/owned".to_string(),
                name: "Owned".to_string(),
                description: "Current catalog".to_string(),
            },
            WorkflowPanelItem {
                uuid: "wf/error".to_string(),
                name: "Failing".to_string(),
                description: "Current catalog".to_string(),
            },
        ],
        ..WorkflowPanel::default()
    };
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: "/workflow:wf/stale do not send".to_string(),
            attachments: Vec::new(),
        },
    );
    assert!(!runner.has_active_stream(), "stale UUID must not start a stream");
    assert!(
        requests.recv_timeout(Duration::from_millis(100)).is_err(),
        "stale UUID must add zero Gateway requests"
    );

    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: "/workflow:wf/owned deploy safely".to_string(),
            attachments: Vec::new(),
        },
    );
    assert_eq!(runner.active_stream_kind(), Some("workflow"));
    drain_until(&mut runner, &mut app, |runner, _| !runner.has_active_stream());

    let first =
        requests.recv_timeout(Duration::from_secs(3)).expect("record owned Workflow request");
    assert_eq!(first.path, "/v1/workflow/applications/wf%2Fowned/chat-messages");
    let body: serde_json::Value = serde_json::from_str(first.body.as_str()).expect("request JSON");
    assert_eq!(body["query"], "deploy safely");
    assert_eq!(body["user"], "runner-session");
    assert_eq!(body["response_mode"], "streaming");

    let ownership = runner.last_workflow_run().expect("Workflow ownership should survive terminal");
    assert_eq!(ownership.request_id(), runner.last_workflow_request_id().unwrap());
    assert_eq!(ownership.workflow_uuid(), "wf/owned");
    assert_eq!(ownership.task_id(), Some("task-1"));
    assert_eq!(ownership.workflow_run_id(), Some("run-1"));
    assert_eq!(ownership.conversation_id(), Some("conversation-1"));
    assert_eq!(ownership.pause().map(|pause| pause.node_id.as_str()), Some("approval"));
    assert_eq!(ownership.terminal_events(), 1, "workflow_finished and message_end deduplicate");

    let messages = &app.state().messages;
    assert!(messages.iter().any(|message| match message {
        super::model::UiMessage::Progress(progress) => progress.label.contains("Prepare"),
        _ => false,
    }));
    assert!(messages.iter().any(|message| match message {
        super::model::UiMessage::Assistant(message) => message.text.contains("deployed"),
        _ => false,
    }));
    assert!(messages.iter().any(|message| match message {
        super::model::UiMessage::System(message) => message.text.contains("custom_progress"),
        _ => false,
    }));

    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: "/workflow:wf/error fail visibly".to_string(),
            attachments: Vec::new(),
        },
    );
    drain_until(&mut runner, &mut app, |runner, _| !runner.has_active_stream());
    let second =
        requests.recv_timeout(Duration::from_secs(3)).expect("record failing Workflow request");
    assert_eq!(second.path, "/v1/workflow/applications/wf%2Ferror/chat-messages");
    assert!(app.state().messages.iter().any(|message| match message {
        super::model::UiMessage::Error(message) => message.message.contains("workflow exploded"),
        _ => false,
    }));
    assert_eq!(runner.last_workflow_run().unwrap().terminal_events(), 1);

    server.join().expect("workflow gateway fixture");
}

#[test]
fn runner_browses_and_reads_workspace_files_without_starting_chat() {
    let (endpoint, requests, server) = workspace_file_runner_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/files src".to_string(), attachments: Vec::new() },
    );
    assert!(!runner.has_active_stream(), "/files must not start a chat stream");
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(3)).expect("record workspace listing"),
        "/v1/file?directory=%2Fworkspace&path=src"
    );
    drain_until(&mut runner, &mut app, |_, app| render_widget(app).contains("lib.rs"));

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: WORKSPACE_FILES_SURFACE.to_string(),
            item_id: "file:src/lib.rs".to_string(),
        },
    );
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(3)).expect("record workspace read"),
        "/v1/file/read"
    );
    server.join().expect("workspace file runner fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        let rendered = render_widget(app);
        rendered.contains("src/lib.rs") && rendered.contains("pub fn gateway_file")
    });
}

#[test]
fn runner_discards_stale_workspace_file_listings() {
    let (endpoint, requests, server) = out_of_order_workspace_files_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    for path in ["old", "new"] {
        runner.execute_action(
            &mut app,
            V3Action::Submit { text: format!("/files {path}"), attachments: Vec::new() },
        );
    }
    let requests = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("record concurrent workspace listings");
    assert_eq!(requests.len(), 2);
    server.join().expect("out-of-order workspace files fixture");

    drain_until(&mut runner, &mut app, |_, app| render_widget(app).contains("new.rs"));
    thread::sleep(Duration::from_millis(80));
    runner.drain(&mut app);
    let rendered = render_widget(&app);
    assert!(rendered.contains("new.rs"));
    assert!(!rendered.contains("old.rs"));
}

#[test]
fn runner_loads_owned_cron_history_without_cross_source_loss() {
    let (endpoint, requests, server) = cron_history_runner_fixture();
    let mut app = test_app(endpoint);
    let job = runner_cron_job("cron/owned");
    app.state_mut().orchestration.cron_jobs = vec![CronJobSnapshot::from_dto(&job)];
    app.state_mut().orchestration.schedules = SchedulePanel {
        items: vec![SchedulePanelItem {
            id: job.id.clone(),
            name: job.name.clone().expect("Cron name"),
            schedule: job.expression.clone(),
            next_run: job.next_run.clone(),
            last_status: job.last_status.clone(),
            enabled: job.enabled,
            history: Vec::new(),
            history_error: None,
        }],
        ..SchedulePanel::default()
    };
    app.state_mut().orchestration.workflows.error = Some("existing workflow warning".to_string());
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/schedule".to_string(), attachments: Vec::new() },
    );
    assert!(render_widget(&app).contains("Nightly cron/owned"));

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: SCHEDULE_SELECTION_SURFACE.to_string(),
            item_id: "cron/stale".to_string(),
        },
    );
    drain_until(&mut runner, &mut app, |_, app| {
        transcript_text(app).contains("current Cron snapshot")
    });
    assert!(
        requests.recv_timeout(Duration::from_millis(100)).is_err(),
        "stale Cron selection must issue zero Gateway requests"
    );

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: SCHEDULE_SELECTION_SURFACE.to_string(),
            item_id: "cron/owned".to_string(),
        },
    );
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(3)).expect("record Cron runs request"),
        "/v1/cron/cron%2Fowned/runs"
    );
    server.join().expect("Cron history fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        transcript_text(app).contains("nightly complete")
    });
    assert_eq!(
        app.state().orchestration.workflows.error.as_deref(),
        Some("existing workflow warning"),
        "Cron source results must not replace Workflow state"
    );
}

#[test]
fn runner_loads_owned_scheduler_task_detail() {
    let (endpoint, requests, server) = task_detail_runner_fixture();
    let mut app = test_app(endpoint);
    app.state_mut().orchestration.scope = Some(
        AgentSchedulerScopeDto::for_default_tenant(ProjectId::from("project-one"))
            .expect("task scope"),
    );
    app.state_mut().orchestration.tasks = BackgroundTaskPanel {
        tasks: vec![BackgroundTask {
            id: "task/owned".to_string(),
            session_id: "project-one".to_string(),
            kind: BackgroundTaskKind::Agent,
            title: "Owned scheduler task".to_string(),
            status: BackgroundTaskStatus::Running,
            canonical_status: CanonicalTaskStatus::Running,
            detail: "Running".to_string(),
            log: Vec::new(),
        }],
        ..BackgroundTaskPanel::default()
    };
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/tasks".to_string(), attachments: Vec::new() },
    );
    assert!(render_widget(&app).contains("Owned scheduler task"));

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: TASK_SELECTION_SURFACE.to_string(),
            item_id: "task/foreign".to_string(),
        },
    );
    drain_until(&mut runner, &mut app, |_, app| {
        transcript_text(app).contains("current project snapshot")
    });
    assert!(
        requests.recv_timeout(Duration::from_millis(100)).is_err(),
        "foreign task selection must issue zero Gateway requests"
    );

    runner.execute_action(
        &mut app,
        V3Action::Select {
            surface: TASK_SELECTION_SURFACE.to_string(),
            item_id: "task/owned".to_string(),
        },
    );
    let mut paths =
        requests.recv_timeout(Duration::from_secs(3)).expect("record task detail requests");
    paths.sort();
    assert_eq!(
        paths,
        vec![
            "/v1/agent-scheduler/projects/project-one/task-graph",
            "/v1/agent-scheduler/projects/project-one/tasks/task%2Fowned/artifacts",
            "/v1/agent-scheduler/projects/project-one/tasks/task%2Fowned/logs",
        ]
    );
    server.join().expect("task detail fixture");
    drain_until(&mut runner, &mut app, |_, app| {
        let transcript = transcript_text(app);
        transcript.contains("completed safely") && transcript.contains("report.txt")
    });
}

fn test_app(endpoint: GatewayEndpoint) -> V3GatewayApp {
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("runner-session".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("runner-session".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    V3GatewayApp::new(runtime, state)
}

fn drain_until(
    runner: &mut ActiveRunner,
    app: &mut V3GatewayApp,
    complete: impl Fn(&ActiveRunner, &V3GatewayApp) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !complete(runner, app) {
        runner.drain(app);
        assert!(
            Instant::now() < deadline,
            "active runner did not settle before deadline; rendered: {}",
            render_widget(app)
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn two_turn_gateway_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind two-turn gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut stream_count = 0;
        let mut save_count = 0;
        while stream_count < 2 || save_count < 2 {
            let (mut stream, _) = listener.accept().expect("accept gateway request");
            let request = read_request(&mut stream);
            sender.send(request.path.clone()).expect("record gateway request");
            if request.path == "/v1/chat/stream?directory=%2Fworkspace" {
                stream_count += 1;
                let body = format!(
                    "id: {stream_count}\ndata: {{\"type\":\"chat.done\",\"finish_reason\":\"stop\"}}\n\n"
                );
                write_response(&mut stream, "text/event-stream", body.as_str());
            } else if request.path.starts_with("/v1/session/runner-session/ui") {
                save_count += 1;
                write_response(&mut stream, "application/json", "true");
            } else {
                panic!("unexpected gateway request: {}", request.path);
            }
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

#[derive(Debug)]
struct InterruptRequests {
    stream_request_id: String,
    stop_path: String,
}

fn interrupt_gateway_fixture()
-> (GatewayEndpoint, mpsc::Receiver<InterruptRequests>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind interrupt gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept stream request");
        let request = read_request(&mut stream);
        assert_eq!(request.path, "/v1/chat/stream?directory=%2Fworkspace");
        let stream_request_id = request
            .header("x-vw-stream-request-id")
            .expect("resumable stream request id")
            .to_string();
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\nid: 1\ndata: {\"type\":\"chat.delta\",\"delta\":\"working\"}\n\n",
            )
            .expect("write streaming response");
        stream.flush().expect("flush streaming response");

        let (mut stop_stream, _) = listener.accept().expect("accept stop request");
        let stop = read_request(&mut stop_stream);
        write_response(&mut stop_stream, "application/json", r#"{"cancelled":true}"#);
        sender
            .send(InterruptRequests { stream_request_id, stop_path: stop.path })
            .expect("record stop request");
        drop(stream);

        let (mut save_stream, _) = listener.accept().expect("accept snapshot request");
        let save = read_request(&mut save_stream);
        assert!(save.path.starts_with("/v1/session/runner-session/ui"));
        write_response(&mut save_stream, "application/json", "true");
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn out_of_order_file_search_fixture()
-> (GatewayEndpoint, mpsc::Receiver<Vec<String>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind file search gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut old_stream = None;
        let mut new_stream = None;
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept file search request");
            let request = read_request(&mut stream);
            if request.path.contains("query=old") {
                old_stream = Some(stream);
            } else if request.path.contains("query=new") {
                new_stream = Some(stream);
            } else {
                panic!("unexpected file search request: {}", request.path);
            }
            requests.push(request.path);
        }
        sender.send(requests).expect("record file search requests");

        write_response(
            &mut new_stream.expect("new file search request"),
            "application/json",
            r#"["src/new.rs"]"#,
        );
        thread::sleep(Duration::from_millis(100));
        write_response(
            &mut old_stream.expect("old file search request"),
            "application/json",
            r#"["src/old.rs"]"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn failing_file_search_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>)
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind failing file search fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failing file search request");
        let request = read_request(&mut stream);
        sender.send(request.path).expect("record failing file search request");
        write_status_response(
            &mut stream,
            "503 Service Unavailable",
            "application/json",
            r#"{"message":"index unavailable"}"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn session_diff_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    session_diff_response_fixture(
        "200 OK",
        r#"[{"file":"src/lib.rs","before":"old value\n","after":"new value\n","additions":1,"deletions":1,"status":"modified"}]"#,
        Duration::from_millis(100),
    )
}

fn session_diff_response_fixture(
    status: &'static str,
    body: &'static str,
    delay: Duration,
) -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind session diff gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept session diff request");
        let request = read_request(&mut stream);
        sender.send(request.path).expect("record session diff request");
        thread::sleep(delay);
        write_status_response(&mut stream, status, "application/json", body);
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn out_of_order_doctor_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>)
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind doctor gateway fixture");
    let port = listener.local_addr().expect("doctor fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stale_stream, _) = listener.accept().expect("accept stale doctor request");
        let stale_request = read_request(&mut stale_stream);
        sender.send(stale_request.path).expect("record stale doctor request");

        let (mut latest_stream, _) = listener.accept().expect("accept latest doctor request");
        let latest_request = read_request(&mut latest_stream);
        sender.send(latest_request.path).expect("record latest doctor request");

        write_response(
            &mut latest_stream,
            "application/json",
            r#"{"results":[{"severity":"ok","category":"gateway","message":"latest result"}],"summary":{"ok":1,"warnings":0,"errors":0}}"#,
        );
        thread::sleep(Duration::from_millis(100));
        write_response(
            &mut stale_stream,
            "application/json",
            r#"{"results":[{"severity":"error","category":"gateway","message":"stale result"}],"summary":{"ok":0,"warnings":0,"errors":1}}"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn doctor_error_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind doctor error fixture");
    let port = listener.local_addr().expect("doctor error fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failed doctor request");
        let request = read_request(&mut stream);
        sender.send(request.path).expect("record failed doctor request");
        write_status_response(
            &mut stream,
            "503 Service Unavailable",
            "application/json",
            r#"{"message":"token=super-secret Authorization=Bearer super-secret"}"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn workflow_gateway_fixture()
-> (GatewayEndpoint, mpsc::Receiver<RecordedRequest>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind workflow gateway fixture");
    let port = listener.local_addr().expect("workflow fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for body in [
            concat!(
                "data: {\"event\":\"workflow_started\",\"task_id\":\"task-1\",\"workflow_run_id\":\"run-1\",\"data\":{}}\n\n",
                "data: {\"event\":\"node_started\",\"task_id\":\"task-1\",\"workflow_run_id\":\"run-1\",\"data\":{\"title\":\"Prepare\"}}\n\n",
                "data: {\"event\":\"text_chunk\",\"task_id\":\"task-1\",\"workflow_run_id\":\"run-1\",\"data\":{\"text\":\"deployed\"}}\n\n",
                "data: {\"event\":\"node_finished\",\"task_id\":\"task-1\",\"workflow_run_id\":\"run-1\",\"data\":{\"title\":\"Prepare\"}}\n\n",
                "data: {\"event\":\"custom_progress\",\"percent\":50}\n\n",
                "data: {\"event\":\"workflow_finished\",\"task_id\":\"task-1\",\"workflow_run_id\":\"run-1\",\"data\":{\"status\":\"running\",\"pause\":{\"node_id\":\"approval\",\"title\":\"Review\",\"form_token\":\"secret-token\",\"form\":{},\"actions\":[]}}}\n\n",
                "data: {\"event\":\"message_end\",\"id\":\"event-1\",\"task_id\":\"task-1\",\"message_id\":\"message-1\",\"conversation_id\":\"conversation-1\",\"metadata\":{}}\n\n"
            ),
            concat!(
                "data: {\"event\":\"workflow_started\",\"task_id\":\"task-2\",\"workflow_run_id\":\"run-2\",\"data\":{}}\n\n",
                "data: {\"event\":\"error\",\"task_id\":\"task-2\",\"status\":500,\"code\":\"workflow_failed\",\"message\":\"workflow exploded\"}\n\n"
            ),
        ] {
            let (mut stream, _) = listener.accept().expect("accept Workflow stream");
            let request = read_request(&mut stream);
            sender.send(request).expect("record Workflow request");
            write_response(&mut stream, "text/event-stream", body);

            let (mut save_stream, _) = listener.accept().expect("accept Workflow snapshot save");
            let save = read_request(&mut save_stream);
            assert!(save.path.starts_with("/v1/session/runner-session/ui"));
            write_response(&mut save_stream, "application/json", "true");
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn workspace_file_runner_fixture()
-> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind workspace file runner fixture");
    let port = listener.local_addr().expect("workspace file runner address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut list_stream, _) = listener.accept().expect("accept workspace listing");
        let list = read_request(&mut list_stream);
        sender.send(list.path).expect("record workspace listing");
        write_response(
            &mut list_stream,
            "application/json",
            r#"[{"path":"src/lib.rs","name":"lib.rs","type":"file"}]"#,
        );

        let (mut read_stream, _) = listener.accept().expect("accept workspace read");
        let read = read_request(&mut read_stream);
        sender.send(read.path).expect("record workspace read");
        write_response(
            &mut read_stream,
            "application/json",
            r#"{"root_directory":"/workspace","path":"src/lib.rs","content":"pub fn gateway_file() {}"}"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn cron_history_runner_fixture() -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>)
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind Cron history fixture");
    let port = listener.local_addr().expect("Cron history address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept Cron history request");
        let request = read_request(&mut stream);
        sender.send(request.path).expect("record Cron history request");
        write_response(
            &mut stream,
            "application/json",
            r#"{"runs":[{"id":7,"job_id":"cron/owned","started_at":"start","finished_at":"finish","status":"ok","output":"nightly complete","duration_ms":12}]}"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn task_detail_runner_fixture()
-> (GatewayEndpoint, mpsc::Receiver<Vec<String>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind task detail fixture");
    let port = listener.local_addr().expect("task detail address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut paths = Vec::new();
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().expect("accept task detail request");
            let request = read_request(&mut stream);
            let body = if request.path.ends_with("/task-graph") {
                r#"{"project_id":"project-one","edges":[],"tasks":[{"task_id":"task/owned","status":"running","readiness":"ready"}]}"#
            } else if request.path.ends_with("/logs") {
                r#"{"task_id":"task/owned","logs":[{"task_id":"task/owned","seq_no":1,"timestamp_ms":2,"to_status_key":"completed","status_to":"completed","message":"completed safely"}]}"#
            } else if request.path.ends_with("/artifacts") {
                r#"{"task_id":"task/owned","artifacts":[{"task_id":"task/owned","run_id":"run-1","kind":"report","name":"report.txt","path":"/tmp/report.txt","payload":{"content":"must not render"},"created_at_ms":3}]}"#
            } else {
                panic!("unexpected task detail request: {}", request.path);
            };
            paths.push(request.path);
            write_response(&mut stream, "application/json", body);
        }
        sender.send(paths).expect("record task detail requests");
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn runner_cron_job(id: &str) -> CronJobDto {
    CronJobDto {
        id: id.to_string(),
        name: Some(format!("Nightly {id}")),
        job_type: "command".to_string(),
        schedule_kind: "cron".to_string(),
        expression: "0 0 * * *".to_string(),
        at: None,
        every_ms: None,
        command: "deploy".to_string(),
        prompt: None,
        model: Some("auto".to_string()),
        agent: None,
        acp_agent: None,
        project_path: Some("/workspace".to_string()),
        wake: true,
        fallbacks: Vec::new(),
        full_access: false,
        task_pool: false,
        delivery_mode: "ui".to_string(),
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: true,
        delete_after_run: false,
        next_run: "tomorrow".to_string(),
        last_run: None,
        last_status: Some("ok".to_string()),
        last_output: None,
        enabled: true,
    }
}

fn out_of_order_workspace_files_fixture()
-> (GatewayEndpoint, mpsc::Receiver<Vec<String>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stale workspace files fixture");
    let port = listener.local_addr().expect("stale workspace files address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut old_stream = None;
        let mut new_stream = None;
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept workspace listing");
            let request = read_request(&mut stream);
            if request.path.ends_with("path=old") {
                old_stream = Some(stream);
            } else if request.path.ends_with("path=new") {
                new_stream = Some(stream);
            } else {
                panic!("unexpected workspace listing: {}", request.path);
            }
            requests.push(request.path);
        }
        sender.send(requests).expect("record concurrent workspace listings");
        write_response(
            &mut new_stream.expect("new workspace listing"),
            "application/json",
            r#"[{"path":"new/new.rs","name":"new.rs","type":"file"}]"#,
        );
        thread::sleep(Duration::from_millis(100));
        write_response(
            &mut old_stream.expect("old workspace listing"),
            "application/json",
            r#"[{"path":"old/old.rs","name":"old.rs","type":"file"}]"#,
        );
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn render_widget(app: &V3GatewayApp) -> String {
    let area = Rect::new(0, 0, 80, 18);
    let mut buffer = Buffer::empty(area);
    app.widget().render(area, &mut buffer);
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}

fn transcript_text(app: &V3GatewayApp) -> String {
    app.transcript_lines(240, true).iter().map(ToString::to_string).collect::<String>()
}

struct RecordedRequest {
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl RecordedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn read_request(stream: &mut TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).expect("read gateway request");
        assert!(read > 0, "gateway request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
    let mut lines = headers.lines();
    let path = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("gateway request path")
        .to_string();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_string(), value.trim().to_string()))
        .collect::<Vec<_>>();
    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or_default();
    let body_start = header_end.saturating_add(4);
    while bytes.len().saturating_sub(body_start) < content_length {
        let read = stream.read(&mut buffer).expect("read gateway request body");
        assert!(read > 0, "gateway request closed before body");
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body = String::from_utf8(bytes[body_start..body_start + content_length].to_vec())
        .expect("gateway request body utf8");
    RecordedRequest { path, headers, body }
}

fn write_response(stream: &mut TcpStream, content_type: &str, body: &str) {
    write_status_response(stream, "200 OK", content_type, body);
}

fn write_status_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write gateway response");
}
