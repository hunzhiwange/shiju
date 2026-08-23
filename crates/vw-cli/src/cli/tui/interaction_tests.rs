use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;
use crossterm::event::{KeyCode, KeyEvent};
use vw_gateway_client::{GatewayChatStreamEvent, GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision, V3UserInputAnswer};

use super::app::{ActiveRunner, V3GatewayApp};
use super::model::UiOverlay;
use super::model::permission::PermissionRequest;
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

#[test]
fn task_state_change_refreshes_current_interactions() {
    let (endpoint, requests, server) = interaction_snapshot_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    app.accept_gateway_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.question_raised",
        "session_id": "other-session"
    })));
    runner.drain(&mut app);
    assert!(!runner.interaction_in_flight(), "cross-session events must be ignored");

    app.accept_gateway_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.todo_updated",
        "session_id": "runner-session"
    })));
    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.interaction_in_flight()
            && app.state().tasks.pending_questions.len() == 1
            && app.state().tasks.todo_overlay.as_ref().is_some_and(|todo| todo.items.len() == 1)
    });

    let permission = active_permission(&app);
    assert_eq!(permission.session_id, "runner-session");
    assert_eq!(app.state().tasks.pending_questions[0].session_id, "runner-session");
    assert!(app.widget().has_overlay());

    let recorded = requests.try_iter().collect::<Vec<_>>();
    assert_eq!(recorded.len(), 3);
    assert!(recorded.iter().any(|request| request.path.starts_with("/v1/permission")));
    assert!(recorded.iter().any(|request| request.path == "/v1/question"));
    assert!(
        recorded
            .iter()
            .any(|request| request.path.starts_with("/v1/session/runner-session/todo"))
    );
    server.join().expect("interaction snapshot fixture");
}

#[test]
fn permission_and_question_replies_survive_gateway_failure() {
    let (endpoint, requests, server) = failed_replies_fixture();
    let mut app = test_app(endpoint);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.request_interaction_refresh(&mut app);
    drain_until(&mut runner, &mut app, |runner, app| {
        !runner.interaction_in_flight() && active_permission(app).id == "permission-1"
    });

    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval {
            request_id: "permission-1".to_string(),
            decision: V3ApprovalDecision::Allow,
        },
    );
    drain_until(&mut runner, &mut app, |runner, _| !runner.interaction_in_flight());
    assert_eq!(active_permission(&app).id, "permission-1");
    assert!(app.widget().has_overlay(), "failed permission reply must restore the overlay");

    app.state_mut().overlays.stack.retain(|overlay| !matches!(overlay, UiOverlay::Permissions(_)));
    app.show_next_interaction();
    runner.execute_action(
        &mut app,
        V3Action::AnswerUserInput {
            request_id: "question-1".to_string(),
            answers: vec![V3UserInputAnswer {
                question_id: "0".to_string(),
                values: vec!["Safe".to_string()],
            }],
        },
    );
    drain_until(&mut runner, &mut app, |runner, _| !runner.interaction_in_flight());
    assert_eq!(app.state().tasks.pending_questions[0].request_id, "question-1");
    assert!(app.widget().has_overlay(), "failed question reply must restore the overlay");

    let reject = app.widget_mut().handle_key_event(KeyEvent::from(KeyCode::Esc));
    runner.execute_action(&mut app, reject);
    drain_until(&mut runner, &mut app, |runner, _| !runner.interaction_in_flight());
    assert_eq!(app.state().tasks.pending_questions[0].request_id, "question-1");
    assert!(app.widget().has_overlay(), "failed question rejection must restore the overlay");

    let recorded = requests.try_iter().collect::<Vec<_>>();
    let permission_reply = recorded
        .iter()
        .find(|request| request.path.contains("/permission/permission-1/reply"))
        .expect("permission reply request");
    assert!(permission_reply.body.contains("\"reply\":\"once\""));
    let question_reply = recorded
        .iter()
        .find(|request| request.path == "/v1/question/question-1/reply")
        .expect("question reply request");
    assert!(question_reply.body.contains("\"answers\":[[\"Safe\"]]"));
    assert!(
        recorded
            .iter()
            .any(|request| request.path == "/v1/question/question-1/reject")
    );
    server.join().expect("failed replies fixture");
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

fn active_permission(app: &V3GatewayApp) -> &PermissionRequest {
    let UiOverlay::Permissions(overlay) = app
        .state()
        .overlays
        .stack
        .iter()
        .find(|overlay| matches!(overlay, UiOverlay::Permissions(_)))
        .expect("permission overlay")
    else {
        unreachable!("matched permission overlay")
    };
    overlay.queue.current().expect("pending permission")
}

fn drain_until(
    runner: &mut ActiveRunner,
    app: &mut V3GatewayApp,
    complete: impl Fn(&ActiveRunner, &V3GatewayApp) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !complete(runner, app) {
        runner.drain(app);
        assert!(Instant::now() < deadline, "interaction runner did not settle before deadline");
        thread::sleep(Duration::from_millis(10));
    }
}

fn interaction_snapshot_fixture(
) -> (GatewayEndpoint, mpsc::Receiver<RecordedRequest>, thread::JoinHandle<()>) {
    scripted_gateway(3, false)
}

fn failed_replies_fixture(
) -> (GatewayEndpoint, mpsc::Receiver<RecordedRequest>, thread::JoinHandle<()>) {
    scripted_gateway(6, true)
}

fn scripted_gateway(
    request_count: usize,
    fail_replies: bool,
) -> (GatewayEndpoint, mpsc::Receiver<RecordedRequest>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind interaction fixture");
    let port = listener.local_addr().expect("interaction fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..request_count {
            let (mut stream, _) = listener.accept().expect("accept interaction request");
            let request = read_request(&mut stream);
            let path = request.path.clone();
            sender.send(request).expect("record interaction request");
            if fail_replies && (path.contains("/reply") || path.contains("/reject")) {
                write_response(&mut stream, "500 Internal Server Error", r#"{"error":"forced"}"#);
            } else if path.starts_with("/v1/permission") {
                write_response(
                    &mut stream,
                    "200 OK",
                    r#"[{"id":"permission-1","session_id":"runner-session","permission":"shell","patterns":["git status"],"always":[],"metadata":{}},{"id":"foreign-permission","session_id":"other-session","permission":"shell","patterns":[],"always":[],"metadata":{}}]"#,
                );
            } else if path == "/v1/question" {
                write_response(
                    &mut stream,
                    "200 OK",
                    r#"[{"id":"question-1","sessionID":"runner-session","questions":[{"question":"Choose","header":"Mode","options":[{"label":"Safe","description":"Read only"}]}]},{"id":"foreign-question","sessionID":"other-session","questions":[]}]"#,
                );
            } else if path.starts_with("/v1/session/runner-session/todo") {
                write_response(
                    &mut stream,
                    "200 OK",
                    r#"[{"id":"todo-1","content":"Review","status":"pending","priority":"high"}]"#,
                );
            } else {
                panic!("unexpected interaction request: {path}");
            }
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

#[derive(Debug)]
struct RecordedRequest {
    path: String,
    body: String,
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
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or_default();
    let body_start = header_end.saturating_add(4);
    while bytes.len().saturating_sub(body_start) < content_length {
        let read = stream.read(&mut buffer).expect("read gateway request body");
        assert!(read > 0, "gateway request closed before body");
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body = String::from_utf8_lossy(&bytes[body_start..body_start + content_length]).into_owned();
    RecordedRequest { path, body }
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write gateway response");
}
