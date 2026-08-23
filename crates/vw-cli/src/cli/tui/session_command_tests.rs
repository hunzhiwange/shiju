use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use serde_json::{Value, json};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::{ActiveRunner, V3GatewayApp};
use super::commands::{CommandRoute, route_slash_submission};
use super::model::{UiConfirmIntent, UiOverlay};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

#[test]
fn session_mutations_use_confirmed_gateway_contracts() {
    assert_eq!(
        route_slash_submission("/rewind current-user").expect("route reset alias"),
        Some(CommandRoute::ResetSession { message_id: Some("current-user".to_string()) })
    );
    assert_eq!(
        route_slash_submission("/summarize current-assistant").expect("route compact alias"),
        Some(CommandRoute::CompactSession {
            message_id: Some("current-assistant".to_string())
        })
    );

    let cases = [
        SuccessCase::Rename,
        SuccessCase::Fork,
        SuccessCase::Reset,
        SuccessCase::Compact,
        SuccessCase::Archive,
    ];
    for case in cases {
        case.assert_contract();
    }

    for command in ["/reset current-user", "/compact current-assistant", "/archive"] {
        let (endpoint, requests, server) = gateway_fixture(Vec::new());
        let mut app = test_app(endpoint);
        let before = app.state().clone();
        let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

        runner.execute_action(&mut app, submit(command));
        let confirmation = active_confirmation(&app);
        runner.execute_action(
            &mut app,
            V3Action::ResolveApproval {
                request_id: confirmation.0,
                decision: V3ApprovalDecision::Cancel,
            },
        );

        assert_eq!(app.state(), &before, "cancel must restore the pre-command state for {command}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty | TryRecvError::Disconnected)));
        server.join().expect("cancel fixture");
    }

    for command in ["/fork missing-message", "/reset missing-message", "/compact missing-message"] {
        let (endpoint, requests, server) = gateway_fixture(Vec::new());
        let mut app = test_app(endpoint);
        let before = app.state().clone();
        let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

        runner.execute_action(&mut app, submit(command));

        assert_eq!(app.state(), &before, "invalid message id must not mutate state for {command}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty | TryRecvError::Disconnected)));
        server.join().expect("invalid message id fixture");
    }
}

#[test]
fn failed_session_mutation_keeps_current_snapshot() {
    for case in [
        FailureCase::Rename,
        FailureCase::Fork,
        FailureCase::Reset,
        FailureCase::Compact,
        FailureCase::Archive,
    ] {
        let responses = match case {
            FailureCase::Archive => vec![
                GatewayResponse::ok(json!([])),
                GatewayResponse::error("forced archive failure"),
            ],
            _ => vec![GatewayResponse::error("forced mutation failure")],
        };
        let (endpoint, requests, server) = gateway_fixture(responses);
        let mut app = test_app(endpoint);
        app.widget_mut().replace_draft("unfinished draft");
        let before = app.state().clone();
        let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

        runner.execute_action(&mut app, submit(case.command()));
        if case.requires_confirmation() {
            let confirmation = active_confirmation(&app);
            runner.execute_action(
                &mut app,
                V3Action::ResolveApproval {
                    request_id: confirmation.0,
                    decision: V3ApprovalDecision::Allow,
                },
            );
        }

        assert_eq!(app.state(), &before, "failed {case:?} must keep the canonical snapshot");
        assert_eq!(app.runtime().session_id(), Some("current"));
        assert_eq!(app.widget().draft(), "unfinished draft");
        assert!(requests.recv().is_ok(), "failed {case:?} must reach its Gateway contract");
        server.join().expect("failed session mutation fixture");
    }
}

#[derive(Clone, Copy, Debug)]
enum SuccessCase {
    Rename,
    Fork,
    Reset,
    Compact,
    Archive,
}

impl SuccessCase {
    fn assert_contract(self) {
        let (responses, expected_requests, expected_session_id, expected_title) = match self {
            Self::Rename => (
                vec![
                    GatewayResponse::ok(json!({"id": "current"})),
                    GatewayResponse::ok(snapshot_value("current", "Renamed session", "kept")),
                ],
                vec![
                    ExpectedRequest::new(
                        "PATCH",
                        "/v1/session/current?directory=%2Fworkspace",
                        json!({"title": "Renamed session"}),
                    ),
                    ExpectedRequest::get("/v1/session/current/ui?directory=%2Fworkspace"),
                ],
                "current",
                "Renamed session",
            ),
            Self::Fork => (
                vec![
                    GatewayResponse::ok(json!({"id": "forked"})),
                    GatewayResponse::ok(snapshot_value("forked", "Forked session", "forked")),
                ],
                vec![
                    ExpectedRequest::new(
                        "POST",
                        "/v1/session/current/fork?directory=%2Fworkspace",
                        json!({}),
                    ),
                    ExpectedRequest::get("/v1/session/forked/ui?directory=%2Fworkspace"),
                ],
                "forked",
                "Forked session",
            ),
            Self::Reset => (
                vec![
                    GatewayResponse::ok(json!({"id": "current"})),
                    GatewayResponse::ok(snapshot_value("current", "Current", "rewound")),
                ],
                vec![
                    ExpectedRequest::new(
                        "POST",
                        "/v1/session/current/reset?directory=%2Fworkspace",
                        json!({"messageID": "current-user", "revertCode": true}),
                    ),
                    ExpectedRequest::get("/v1/session/current/ui?directory=%2Fworkspace"),
                ],
                "current",
                "Current",
            ),
            Self::Compact => (
                vec![
                    GatewayResponse::ok(json!(true)),
                    GatewayResponse::ok(snapshot_value("current", "Current", "compacted")),
                ],
                vec![
                    ExpectedRequest::new(
                        "POST",
                        "/v1/session/current/summarize?directory=%2Fworkspace",
                        json!({"messageID": "current-assistant"}),
                    ),
                    ExpectedRequest::get("/v1/session/current/ui?directory=%2Fworkspace"),
                ],
                "current",
                "Current",
            ),
            Self::Archive => (
                vec![
                    GatewayResponse::ok(json!(["already-archived"])),
                    GatewayResponse::ok(json!(true)),
                    GatewayResponse::ok(snapshot_value("current", "Current", "archived")),
                ],
                vec![
                    ExpectedRequest::get("/v1/session/archived?directory=%2Fworkspace"),
                    ExpectedRequest::new(
                        "PUT",
                        "/v1/session/archived?directory=%2Fworkspace",
                        json!(["already-archived", "current"]),
                    ),
                    ExpectedRequest::get("/v1/session/current/ui?directory=%2Fworkspace"),
                ],
                "current",
                "Current",
            ),
        };

        let (endpoint, requests, server) = gateway_fixture(responses);
        let mut app = test_app(endpoint);
        let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));
        runner.execute_action(&mut app, submit(self.command()));
        if self.requires_confirmation() {
            let (request_id, intent) = active_confirmation(&app);
            assert_eq!(intent, self.confirm_intent(), "destructive command intent must stay typed");
            runner.execute_action(
                &mut app,
                V3Action::ResolveApproval { request_id, decision: V3ApprovalDecision::Allow },
            );
        }

        assert_eq!(app.state().session.session_id.as_deref(), Some(expected_session_id));
        assert_eq!(app.runtime().session_id(), Some(expected_session_id));
        assert_eq!(app.state().session.title, expected_title);
        for expected in expected_requests {
            assert_eq!(requests.recv().expect("recorded gateway request"), expected);
        }
        server.join().expect("successful session mutation fixture");
    }

    fn command(self) -> &'static str {
        match self {
            Self::Rename => "/rename Renamed session",
            Self::Fork => "/fork",
            Self::Reset => "/reset current-user",
            Self::Compact => "/compact current-assistant",
            Self::Archive => "/archive",
        }
    }

    fn requires_confirmation(self) -> bool {
        matches!(self, Self::Reset | Self::Compact | Self::Archive)
    }

    fn confirm_intent(self) -> UiConfirmIntent {
        match self {
            Self::Reset => UiConfirmIntent::ResetSession {
                message_id: "current-user".to_string(),
            },
            Self::Compact => UiConfirmIntent::CompactSession {
                message_id: "current-assistant".to_string(),
            },
            Self::Archive => UiConfirmIntent::ArchiveSession,
            Self::Rename | Self::Fork => panic!("non-destructive operation has no confirmation"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FailureCase {
    Rename,
    Fork,
    Reset,
    Compact,
    Archive,
}

impl FailureCase {
    fn command(self) -> &'static str {
        match self {
            Self::Rename => "/rename Failed title",
            Self::Fork => "/fork",
            Self::Reset => "/reset current-user",
            Self::Compact => "/compact current-assistant",
            Self::Archive => "/archive",
        }
    }

    fn requires_confirmation(self) -> bool {
        matches!(self, Self::Reset | Self::Compact | Self::Archive)
    }
}

fn active_confirmation(app: &V3GatewayApp) -> (String, UiConfirmIntent) {
    let Some(UiOverlay::Confirm(confirmation)) = app.state().overlays.active() else {
        panic!("expected an active session confirmation")
    };
    assert!(confirmation.destructive);
    (confirmation.request_id.clone(), confirmation.intent.clone())
}

fn submit(text: &str) -> V3Action {
    V3Action::Submit { text: text.to_string(), attachments: Vec::new() }
}

fn test_app(endpoint: GatewayEndpoint) -> V3GatewayApp {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(endpoint).expect("test gateway client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("current".to_string()))
            .with_scope(Some("workspace-write".to_string())),
    );
    let mut state = TuiState::from_chat_session(&snapshot("current", "Current", "kept"));
    state.session.scope = Some("workspace-write".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    V3GatewayApp::new(runtime, state)
}

fn snapshot(id: &str, title: &str, assistant: &str) -> ChatSession {
    ChatSession {
        id: id.to_string(),
        title: title.to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "question".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: assistant.to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![
            Some(format!("{id}-user")),
            Some(format!("{id}-assistant")),
        ],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

fn snapshot_value(id: &str, title: &str, assistant: &str) -> Value {
    serde_json::to_value(snapshot(id, title, assistant)).expect("serialize snapshot")
}

#[derive(Debug, PartialEq)]
struct ExpectedRequest {
    method: String,
    path: String,
    body: Value,
}

impl ExpectedRequest {
    fn new(method: &str, path: &str, body: Value) -> Self {
        Self { method: method.to_string(), path: path.to_string(), body }
    }

    fn get(path: &str) -> Self {
        Self::new("GET", path, Value::Null)
    }
}

struct GatewayResponse {
    status: &'static str,
    body: Value,
}

impl GatewayResponse {
    fn ok(body: Value) -> Self {
        Self { status: "200 OK", body }
    }

    fn error(message: &str) -> Self {
        Self { status: "500 Internal Server Error", body: json!({"error": message}) }
    }
}

fn gateway_fixture(
    responses: Vec<GatewayResponse>,
) -> (GatewayEndpoint, mpsc::Receiver<ExpectedRequest>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind session command fixture");
    let port = listener.local_addr().expect("session command fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept session command request");
            sender.send(read_request(&mut stream)).expect("record session command request");
            let body = response.body.to_string();
            let response = format!(
                "HTTP/1.1 {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                response.status,
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write session command response");
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn read_request(stream: &mut TcpStream) -> ExpectedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).expect("read session command request");
        assert!(read > 0, "gateway request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let mut request_line = headers.lines().next().expect("request line").split_whitespace();
    let method = request_line.next().expect("request method").to_string();
    let path = request_line.next().expect("request path").to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content length"))
        })
        .unwrap_or_default();
    while bytes.len().saturating_sub(header_end) < content_length {
        let read = stream.read(&mut buffer).expect("read session command body");
        assert!(read > 0, "gateway request closed before body");
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body = if content_length == 0 {
        Value::Null
    } else {
        serde_json::from_slice(&bytes[header_end..header_end + content_length])
            .expect("JSON request body")
    };
    ExpectedRequest { method, path, body }
}
