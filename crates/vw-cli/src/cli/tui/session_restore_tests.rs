use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
use vw_tui::V3Action;

use super::app::{
    ActiveRunner, DirectorySwitchBootstrap, V3GatewayApp, initial_tui_state,
    restore_or_create_session_state,
};
use super::bridge::GatewayV3Bridge;
use super::commands::{CommandRoute, SESSION_RESUME_SURFACE, route_selection};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

#[test]
fn resume_candidates_exclude_archived_and_sort_latest_first() {
    let (endpoint, requests, server) = gateway_fixture(vec![
        GatewayResponse::ok(json!([
            {
                "id": "older",
                "title": "Older",
                "updated_ms": 10,
                "message_count": 3,
                "call_count": 0
            },
            {
                "id": "archived",
                "title": "Archived",
                "updated_ms": 30,
                "message_count": 4,
                "call_count": 0
            },
            {
                "id": "newer",
                "title": "Newer",
                "updated_ms": 20,
                "message_count": 2,
                "call_count": 1
            }
        ])),
        GatewayResponse::ok(json!(["archived"])),
    ]);
    let runtime = test_runtime(endpoint, "current");

    let candidates =
        runtime.session_resume_candidates_blocking().expect("load resumable session candidates");

    assert_eq!(
        candidates.iter().map(|candidate| candidate.id.as_str()).collect::<Vec<_>>(),
        vec!["newer", "older"]
    );
    assert_eq!(
        requests.recv().expect("preview request"),
        "/v1/session/ui-previews?directory=%2Fworkspace"
    );
    assert_eq!(
        requests.recv().expect("archived request"),
        "/v1/session/archived?directory=%2Fworkspace"
    );
    server.join().expect("resume candidate fixture");
}

#[test]
fn startup_state_is_empty_before_background_session_create() {
    let runtime = test_runtime(GatewayEndpoint::new("127.0.0.1", 9), "previous");

    let state = initial_tui_state(&runtime);

    assert_eq!(state.session.session_id, None);
    assert!(state.messages.is_empty());
    assert_eq!(state.session.scope.as_deref(), Some("workspace-write"));
    assert_eq!(state.project.workspace_root.as_deref(), Some(Path::new("/workspace")));
}

#[test]
fn background_bootstrap_creates_new_session_without_loading_history() {
    let (endpoint, requests, server) = gateway_fixture(vec![GatewayResponse::ok(json!({
        "id": "created",
        "title": "Fresh session"
    }))]);
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    let mut runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_scope(Some("workspace-write".to_string())),
    );

    let state = restore_or_create_session_state(&mut runtime).expect("create session in worker");

    assert_eq!(state.session.session_id.as_deref(), Some("created"));
    assert!(state.messages.is_empty());
    assert_eq!(
        requests.recv().expect("create request"),
        "/v1/session?directory=%2Fworkspace"
    );
    server.join().expect("background create fixture");
    assert!(requests.try_recv().is_err(), "startup must not request session history");
}

#[test]
fn background_bootstrap_restores_explicit_session_in_current_workspace() {
    let restored = snapshot("explicit", "Recovered", "latest answer");
    let (endpoint, requests, server) = gateway_fixture(vec![GatewayResponse::ok(
        serde_json::to_value(restored).expect("snapshot"),
    )]);
    let mut runtime = test_runtime(endpoint, "explicit");

    let state = restore_or_create_session_state(&mut runtime).expect("restore explicit session");

    assert_eq!(state.session.session_id.as_deref(), Some("explicit"));
    assert_eq!(
        state.to_chat_session().messages.last().map(|message| message.content.as_str()),
        Some("latest answer")
    );
    assert_eq!(
        requests.recv().expect("scoped snapshot request"),
        "/v1/session/explicit/ui?directory=%2Fworkspace"
    );
    server.join().expect("explicit restore fixture");
}

#[test]
fn restored_widget_replays_the_latest_messages() {
    let mut restored = snapshot("restored", "Recovered", "newest answer");
    restored.messages.splice(
        0..0,
        (0..101).map(|index| ChatMessage {
            role: ChatRole::Assistant,
            content: format!("older answer {index}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }),
    );
    restored.message_ids = vec![None; restored.messages.len()];
    let runtime = test_runtime(GatewayEndpoint::new("127.0.0.1", 9), "restored");

    let app = V3GatewayApp::new(runtime, TuiState::from_chat_session(&restored));
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("newest answer"));
    assert!(!rendered.contains("older answer 0"));
}

#[test]
fn restored_tool_result_keeps_tool_name_and_command() {
    let mut restored = snapshot("restored-tool", "Recovered", "done");
    restored.messages.insert(
        0,
        ChatMessage {
            role: ChatRole::Tool,
            content: concat!(
                "tool command\n",
                r#"{"status":"completed","input":"ls -la","output":"file.txt"}"#,
                "\n"
            )
            .to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    );
    restored.message_ids.insert(0, Some("tool-message".to_string()));
    let runtime = test_runtime(GatewayEndpoint::new("127.0.0.1", 9), "restored-tool");

    let app = V3GatewayApp::new(runtime, TuiState::from_chat_session(&restored));
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("› command ls -la"));
    assert!(!rendered.contains("tool completed command"));
}

#[test]
fn resume_selection_uses_the_gateway_session_id() {
    assert_eq!(
        route_selection(SESSION_RESUME_SURFACE, "stable-session-id")
            .expect("route stable session id"),
        CommandRoute::ResumeSession { session_id: "stable-session-id".to_string() }
    );
}

#[test]
fn resume_switch_is_transactional() {
    let (endpoint, requests, server) = gateway_fixture(vec![GatewayResponse {
        status: "500 Internal Server Error",
        body: json!({"error": "forced load failure"}),
    }]);
    let runtime = test_runtime(endpoint, "current");
    let mut state = TuiState::from_chat_session(&snapshot("current", "Current", "keep me"));
    state.session.scope = Some("workspace-write".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    let mut app = V3GatewayApp::new(runtime, state);
    app.widget_mut().replace_draft("unfinished draft");
    let before = app.state().clone();

    let error = app.resume_session("target").expect_err("forced load failure must surface");

    assert!(error.contains("500") || error.contains("forced load failure"));
    assert_eq!(app.state(), &before);
    assert_eq!(app.widget().draft(), "unfinished draft");
    assert_eq!(app.runtime().session_id(), Some("current"));
    assert_eq!(
        requests.recv().expect("snapshot request"),
        "/v1/session/target/ui?directory=%2Fworkspace"
    );
    server.join().expect("failed resume fixture");
}

#[test]
fn resume_switch_commits_state_display_and_runtime_together() {
    let target = snapshot("target", "Recovered", "restored answer");
    let (endpoint, requests, server) =
        gateway_fixture(vec![GatewayResponse::ok(serde_json::to_value(target).expect("snapshot"))]);
    let runtime = test_runtime(endpoint, "current");
    let mut state = TuiState::from_chat_session(&snapshot("current", "Current", "old answer"));
    state.session.scope = Some("workspace-write".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    let mut app = V3GatewayApp::new(runtime, state);
    app.widget_mut().replace_draft("draft survives success");

    app.resume_session("target").expect("resume target session");

    let display = GatewayV3Bridge::session_display_data(app.runtime(), app.state());
    assert_eq!(app.state().session.session_id.as_deref(), Some("target"));
    assert_eq!(display.session_id.as_deref(), Some("target"));
    assert_eq!(app.runtime().session_id(), Some("target"));
    assert_eq!(app.state().session.scope.as_deref(), Some("workspace-write"));
    assert_eq!(app.state().project.workspace_root.as_deref(), Some(Path::new("/workspace")));
    assert_eq!(app.widget().draft(), "draft survives success");
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("Recovered"));
    assert!(rendered.contains("restored answer"));
    assert!(!rendered.contains("old answer"));
    assert_eq!(
        requests.recv().expect("snapshot request"),
        "/v1/session/target/ui?directory=%2Fworkspace"
    );
    server.join().expect("successful resume fixture");
}

#[test]
fn new_session_rebinds_runtime_after_create() {
    let (endpoint, requests, server) = gateway_fixture(vec![GatewayResponse::ok(json!({
        "id": "created",
        "title": "Fresh session"
    }))]);
    let runtime = test_runtime(endpoint, "current");
    let mut state = TuiState::from_chat_session(&snapshot("current", "Current", "old answer"));
    state.session.scope = Some("workspace-write".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    let mut app = V3GatewayApp::new(runtime, state);

    app.create_new_session().expect("create and switch session");

    let display = GatewayV3Bridge::session_display_data(app.runtime(), app.state());
    assert_eq!(app.state().session.session_id.as_deref(), Some("created"));
    assert_eq!(display.session_id.as_deref(), Some("created"));
    assert_eq!(app.runtime().session_id(), Some("created"));
    assert_eq!(app.state().session.title, "Fresh session");
    assert!(app.state().messages.is_empty());
    assert_eq!(app.state().session.scope.as_deref(), Some("workspace-write"));
    assert_eq!(app.state().project.workspace_root.as_deref(), Some(Path::new("/workspace")));
    assert_eq!(requests.recv().expect("create request"), "/v1/session?directory=%2Fworkspace");
    server.join().expect("new session fixture");
}

#[test]
fn cd_switch_commits_only_after_target_bootstrap() {
    let workspace = tempfile::tempdir().expect("workspace");
    let original = workspace.path().join("original");
    let target = workspace.path().join("target");
    std::fs::create_dir_all(&original).expect("original directory");
    std::fs::create_dir_all(&target).expect("target directory");
    let endpoint = GatewayEndpoint::new("127.0.0.1", 9);
    let original_runtime = GatewayUiRuntime::new(
        GatewayClient::new(endpoint.clone()).expect("original client"),
        GatewaySessionSeed::new(original.canonicalize().expect("canonical original"))
            .with_id(Some("original-session".to_string())),
    );
    let mut original_state =
        TuiState::from_chat_session(&snapshot("original-session", "Original", "keep me"));
    original_state.project.workspace_root = Some(original.canonicalize().unwrap());
    let mut app = V3GatewayApp::new(original_runtime, original_state);
    app.widget_mut().replace_draft("unfinished draft");

    let target_runtime = GatewayUiRuntime::new(
        GatewayClient::new(endpoint).expect("target client"),
        GatewaySessionSeed::new(target.canonicalize().expect("canonical target"))
            .with_id(Some("target-session".to_string())),
    );
    let mut target_state =
        TuiState::from_chat_session(&snapshot("target-session", "Target", "new answer"));
    target_state.project.workspace_root = Some(target.canonicalize().unwrap());
    let bootstrap = DirectorySwitchBootstrap::new(target_runtime, target_state);

    assert_eq!(app.runtime().session_id(), Some("original-session"));
    assert_eq!(app.widget().draft(), "unfinished draft");

    app.commit_directory_switch(bootstrap);

    assert_eq!(app.runtime().directory(), target.canonicalize().unwrap());
    assert_eq!(app.runtime().session_id(), Some("target-session"));
    assert_eq!(app.state().session.session_id.as_deref(), Some("target-session"));
    assert_eq!(app.widget().draft(), "unfinished draft");
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("new answer"));
    assert!(!rendered.contains("keep me"));
}

#[test]
fn cd_switch_rejects_busy_runner_and_pending_queue() {
    let workspace = tempfile::tempdir().expect("workspace");
    let target = workspace.path().join("target");
    std::fs::create_dir_all(&target).expect("target directory");
    let (endpoint, _requests, server) = gateway_fixture(vec![GatewayResponse {
        status: "503 Service Unavailable",
        body: json!({"message": "stream fixture complete"}),
    }]);
    let mut app = V3GatewayApp::new(
        test_runtime(endpoint, "current"),
        TuiState::from_chat_session(&snapshot("current", "Current", "keep me")),
    );
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "keep streaming".to_string(), attachments: Vec::new() },
    );
    assert!(runner.has_active_stream());
    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: format!("/cd {}", target.display()),
            attachments: Vec::new(),
        },
    );
    assert_eq!(app.runtime().directory(), Path::new("/workspace"));
    assert!(!runner.directory_switch_in_flight());
    server.join().expect("stream rejection fixture");

    let mut app = V3GatewayApp::new(
        test_runtime(GatewayEndpoint::new("127.0.0.1", 9), "current"),
        TuiState::from_chat_session(&snapshot("current", "Current", "keep me")),
    );
    app.widget_mut().queue_input("later".to_string(), Vec::new());
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: format!("/cd {}", target.display()),
            attachments: Vec::new(),
        },
    );
    assert_eq!(app.runtime().directory(), Path::new("/workspace"));
    assert!(!runner.directory_switch_in_flight());
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("queued prompts are pending"));
}

#[test]
fn cd_bootstrap_failure_keeps_original_runtime_and_draft() {
    let workspace = tempfile::tempdir().expect("workspace");
    let original = workspace.path().join("original");
    let target = workspace.path().join("target");
    std::fs::create_dir_all(&original).expect("original directory");
    std::fs::create_dir_all(&target).expect("target directory");
    let project = json!({
        "project": {
            "id": "target-project",
            "name": "target",
            "directory": target.to_string_lossy(),
            "display_path": target.to_string_lossy(),
            "status": "ready",
            "created_at_ms": 1,
            "updated_at_ms": 2,
            "default_branch": null,
            "current_branch": null,
            "git": {
                "is_repo": false,
                "has_uncommitted_changes": false,
                "ahead": null,
                "behind": null
            },
            "active_worktree_id": null,
            "session_count": 0
        }
    });
    let (endpoint, requests, server) = gateway_fixture(vec![
        GatewayResponse::ok(project),
        GatewayResponse {
            status: "500 Internal Server Error",
            body: json!({"message": "forced target bootstrap failure"}),
        },
    ]);
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(endpoint).expect("client"),
        GatewaySessionSeed::new(original.canonicalize().expect("canonical original"))
            .with_id(Some("original-session".to_string())),
    );
    let mut state =
        TuiState::from_chat_session(&snapshot("original-session", "Original", "keep me"));
    state.project.workspace_root = Some(original.canonicalize().unwrap());
    let mut app = V3GatewayApp::new(runtime, state);
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| Ok(None)));
    let command_draft = format!("/cd {}", target.display());
    runner.execute_action(
        &mut app,
        V3Action::Submit { text: command_draft.clone(), attachments: Vec::new() },
    );
    assert!(runner.directory_switch_in_flight());
    let deadline = Instant::now() + Duration::from_secs(3);
    while runner.directory_switch_in_flight() {
        runner.drain(&mut app);
        assert!(Instant::now() < deadline, "directory switch did not settle");
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(app.runtime().directory(), original.canonicalize().unwrap());
    assert_eq!(app.runtime().session_id(), Some("original-session"));
    assert_eq!(app.widget().draft(), command_draft);
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("failed to switch working directory"));
    assert_eq!(requests.recv().expect("project resolve"), "/v1/projects/resolve");
    let create_request = requests.recv().expect("target session create");
    assert!(create_request.starts_with("/v1/session?directory="));
    assert!(create_request.contains("target"));
    server.join().expect("failed target bootstrap fixture");
}

fn test_runtime(endpoint: GatewayEndpoint, session_id: &str) -> GatewayUiRuntime {
    GatewayUiRuntime::new(
        GatewayClient::new(endpoint).expect("test gateway client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some(session_id.to_string()))
            .with_scope(Some("workspace-write".to_string())),
    )
}

fn snapshot(id: &str, title: &str, assistant: &str) -> ChatSession {
    ChatSession {
        id: id.to_string(),
        title: title.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Assistant,
            content: assistant.to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some(format!("{id}-assistant"))],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

struct GatewayResponse {
    status: &'static str,
    body: serde_json::Value,
}

impl GatewayResponse {
    fn ok(body: serde_json::Value) -> Self {
        Self { status: "200 OK", body }
    }
}

fn gateway_fixture(
    responses: Vec<GatewayResponse>,
) -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind session gateway fixture");
    let port = listener.local_addr().expect("session gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept session gateway request");
            sender.send(read_request_path(&mut stream)).expect("record session gateway request");
            let body = response.body.to_string();
            let response = format!(
                "HTTP/1.1 {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                response.status,
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write session gateway response");
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn read_request_path(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("read session gateway request");
        assert!(read > 0, "gateway request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes)
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("session gateway request path")
        .to_string()
}
