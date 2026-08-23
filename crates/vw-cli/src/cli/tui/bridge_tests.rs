use std::path::PathBuf;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::json;
use vw_gateway_client::{GatewayChatStreamEvent, GatewayClient, GatewayEndpoint};
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
use vw_tui::{V3Action, V3ApprovalDecision, V3Event, V3UserInputAnswer};

use super::app::{V3GatewayApp, restore_or_create_session_state};
use super::bridge::GatewayV3Bridge;
use super::controller::{TuiControllerCommand, TuiOverlayCommand};
use super::model::permission::{
    PermissionDecision, PermissionPreviewKind, PermissionQueue, PermissionRequest,
};
use super::model::{
    PermissionMode, UiMessage, UiOverlay, UiPermissionOverlay, UiQuestionOption, UiQuestionOverlay,
    UiQuestionPrompt, UiTurnTerminal,
};
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::state::TuiState;

#[test]
fn tui_gateway_bridge_preserves_request_and_terminal_semantics() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 9);
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("session-13".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("session-13".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    state.status.model_name = Some("openai/gpt-5.6".to_string());
    state.session_options.permission_mode = PermissionMode::Plan;
    state.session_options.set_full_access(false);
    state.session_options.set_subagent_mode(true);
    state.session_options.delegate_agent = Some("researcher".to_string());
    state.session_options.select_acp_backend(Some("codex".to_string()));

    let display = GatewayV3Bridge::session_display_data(&runtime, &state);
    assert_eq!(display.version, "0.4.0");
    assert_eq!(display.cwd, PathBuf::from("/workspace"));
    assert_eq!(display.gateway_endpoint, "127.0.0.1:9");
    assert_eq!(display.model, "openai/gpt-5.6");
    assert_eq!(display.permissions, "Plan · workspace write");
    assert_eq!(display.session_id.as_deref(), Some("session-13"));
    assert!(!display.resumed);

    let mut app = V3GatewayApp::new(runtime, state);
    assert_eq!(app.runtime().session_id(), Some("session-13"));

    let submit = app
        .handle_action(V3Action::Submit {
            text: "bridge prompt".to_string(),
            attachments: Vec::new(),
        })
        .expect("TUI submit maps through the controller");
    let request = app.request_for_command(&submit).expect("submit builds a gateway request");
    assert_eq!(request.session_id.as_ref().map(AsRef::as_ref), Some("session-13"));
    assert_eq!(request.model.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(request.agent.as_deref(), Some("researcher"));
    assert_eq!(request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(request.allowed_tools, Some(vec!["AgentTool".to_string(), "todowrite".to_string()]));
    assert_eq!(request.acp_allowed_tools, request.allowed_tools);
    let options = request.options.as_ref().expect("ACP and subagent options");
    assert_eq!(options["cwd"], "/workspace");
    assert_eq!(options["acp_force_new_session"], true);
    assert_eq!(options["acp_permission_mode"], "approve-reads");
    assert_eq!(options["allowed_tools"], json!(["AgentTool", "todowrite"]));
    assert_eq!(request.system.as_ref().map(Vec::len), Some(1));

    let cancel = app.handle_action(V3Action::Cancel).expect("cancel action");
    assert_eq!(cancel, TuiControllerCommand::CancelActiveSubmission);

    let mut permission_queue = PermissionQueue::default();
    permission_queue.replace(vec![PermissionRequest {
        id: "permission-13".to_string(),
        session_id: "session-13".to_string(),
        permission: "shell".to_string(),
        patterns: vec!["cargo test".to_string()],
        always_patterns: vec!["cargo *".to_string()],
        preview: PermissionPreviewKind::Shell,
        explanation: Some("run focused test".to_string()),
        debug: None,
    }]);
    app.state_mut().overlays.push(UiOverlay::Permissions(UiPermissionOverlay {
        queue: permission_queue,
        notice: None,
    }));
    let permission = app
        .handle_action(V3Action::ResolveApproval {
            request_id: "permission-13".to_string(),
            decision: V3ApprovalDecision::AllowForSession,
        })
        .expect("approval routes to the permission command");
    assert_eq!(
        permission,
        TuiControllerCommand::Overlay(TuiOverlayCommand::PermissionDecision {
            request_id: "permission-13".to_string(),
            decision: PermissionDecision::Always,
        })
    );

    app.state_mut().tasks.pending_questions.push(UiQuestionOverlay {
        request_id: "question-13".to_string(),
        session_id: "session-13".to_string(),
        prompts: vec![UiQuestionPrompt {
            header: "Target".to_string(),
            question: "Which target?".to_string(),
            options: vec![UiQuestionOption {
                label: "lib".to_string(),
                description: "library target".to_string(),
                preview: None,
            }],
            multiple: false,
            allow_custom_input: false,
        }],
        answers: vec![Vec::new()],
        tool: None,
        selected_index: 0,
    });
    let answer = app
        .handle_action(V3Action::AnswerUserInput {
            request_id: "question-13".to_string(),
            answers: vec![V3UserInputAnswer {
                question_id: "0".to_string(),
                values: vec!["lib".to_string()],
            }],
        })
        .expect("question answer routes to existing question submission");
    let TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionSubmitted(question)) = answer
    else {
        panic!("expected existing question submission command");
    };
    assert_eq!(question.request_id, "question-13");
    assert_eq!(question.answers, vec![vec!["lib".to_string()]]);

    let first = app.accept_gateway_event(GatewayChatStreamEvent::Delta("hello ".to_string()));
    assert!(matches!(first.first(), Some(V3Event::TurnStarted { .. })));
    assert!(matches!(
        first.get(1),
        Some(V3Event::TextDelta { delta, .. }) if delta == "hello "
    ));
    let second = app.accept_gateway_event(GatewayChatStreamEvent::Delta("world".to_string()));
    assert_eq!(second.len(), 1, "turn start must not repeat for later deltas");
    assert!(matches!(
        second.first(),
        Some(V3Event::TextDelta { delta, .. }) if delta == "world"
    ));
    let terminal = app.accept_gateway_event(GatewayChatStreamEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: Some(json!({
            "input_tokens": 11,
            "output_tokens": 7,
            "cached_tokens": 3,
            "reasoning_tokens": 2
        })),
        message_id: Some("assistant-13".to_string()),
        parent_message_id: Some("user-13".to_string()),
    });
    assert!(matches!(terminal.as_slice(), [V3Event::TurnCompleted { .. }]));
    assert_eq!(
        app.state().status.turn_terminal,
        UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) }
    );
    let assistant = app
        .state()
        .messages
        .iter()
        .find_map(|message| match message {
            UiMessage::Assistant(message) => Some(message),
            _ => None,
        })
        .expect("runtime pipeline keeps the assistant turn");
    assert_eq!(assistant.text, "hello world");
    assert_eq!(assistant.usage.input_tokens, 11);
    assert_eq!(assistant.usage.output_tokens, 7);
    assert_eq!(assistant.usage.cached_tokens, 3);
    assert_eq!(assistant.usage.reasoning_tokens, 2);
    assert_eq!(assistant.base.id.as_str(), "gateway:assistant-13");
    assert_eq!(assistant.base.parent_id.as_ref().map(|id| id.as_str()), Some("gateway:user-13"));
    let snapshot = app.state().to_chat_session();
    assert_eq!(snapshot.message_ids.last().and_then(Clone::clone).as_deref(), Some("assistant-13"));

    let display = GatewayV3Bridge::session_display_data(app.runtime(), app.state());
    assert_eq!(display.token_usage.input, 11);
    assert_eq!(display.token_usage.output, 7);
    assert_eq!(display.token_usage.cached, 3);
    assert_eq!(display.token_usage.reasoning, 2);
    assert!(display.resumed);

    let area = Rect::new(0, 0, 80, 18);
    let mut buffer = Buffer::empty(area);
    app.widget().render(area, &mut buffer);
    let rendered = buffer.content.iter().map(|cell| cell.symbol()).collect::<String>();
    assert!(rendered.contains(">_ VibeWindow (v0.4.0)"));
    assert!(rendered.contains("hello world"), "copied ChatWidget must render bridged deltas");
}

#[test]
fn tui_projects_typed_tools_and_hides_known_gateway_lifecycle_events() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 9);
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    let runtime =
        GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")));
    let mut app = V3GatewayApp::new(runtime, TuiState::default());

    let step = app.accept_gateway_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.step_start",
        "step_index": 1,
        "created_ms": 100
    })));
    assert!(matches!(step.as_slice(), [V3Event::TurnStarted { .. }]));

    let tool = app.accept_gateway_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.tool",
        "step_index": 1,
        "call_id": "call-1",
        "name": "shell",
        "kind": "start",
        "status": "running",
        "occurred_ms": 101,
        "payload": {
            "status": "running",
            "title": "Run command",
            "input": {"command": "pwd"}
        }
    })));
    assert!(matches!(
        tool.as_slice(),
        [V3Event::ToolStarted { call_id, tool, command, .. }]
            if call_id == "call-1" && tool == "shell" && command.contains("pwd")
    ));

    let tool_complete = app.accept_gateway_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.tool",
        "step_index": 1,
        "call_id": "call-1",
        "name": "shell",
        "kind": "complete",
        "status": "completed",
        "occurred_ms": 111,
        "payload": {"status": "completed", "output": "/workspace", "exit_code": 0}
    })));
    assert!(matches!(
        tool_complete.as_slice(),
        [V3Event::ToolCompleted { call_id, output, duration_ms, .. }]
            if call_id == "call-1" && output == "/workspace" && *duration_ms == 10
    ));
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("shell"));
    assert!(rendered.contains("pwd"));
    assert!(rendered.contains("/workspace"));
    assert!(!rendered.contains("gateway-tool"));
    assert!(!rendered.contains("duplicate tool"));

    for event in [
        GatewayChatStreamEvent::Other(json!({
            "type": "chat.post_tool_round",
            "step_index": 1
        })),
        GatewayChatStreamEvent::Other(json!({
            "type": "chat.step_finish",
            "step_index": 1,
            "finished_ms": 102,
            "usage": {},
            "finish_reason": "tool"
        })),
    ] {
        assert!(app.accept_gateway_event(event).is_empty());
    }

    let unknown = app.accept_gateway_event(GatewayChatStreamEvent::Other(json!({
        "type": "custom.event"
    })));
    assert!(matches!(
        unknown.as_slice(),
        [V3Event::Unknown { type_name, detail }]
            if type_name == "unknown_gateway_event" && detail.ends_with("custom.event")
    ));
}

#[test]
fn tui_restores_gateway_session_title_history_and_identity() {
    let snapshot = ChatSession {
        id: "session-restored".to_string(),
        title: "Desktop session title".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "existing question".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "existing answer".to_string(),
                model: Some("token-plan/qwen3.7-plus".to_string()),
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![Some("user-1".to_string()), Some("assistant-1".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 10,
        updated_ms: 20,
    };
    let (endpoint, requests, server) = gateway_fixture(vec![
        json!([{
            "id": "session-restored",
            "title": "Desktop session title",
            "updated_ms": 20,
            "message_count": 2,
            "call_count": 0,
            "last_content": "existing answer"
        }]),
        json!("project-scope"),
        serde_json::to_value(&snapshot).expect("snapshot JSON"),
        json!(true),
    ]);
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    let mut runtime =
        GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")));

    let state = restore_or_create_session_state(&mut runtime).expect("restore latest session");
    assert_eq!(state.session.session_id.as_deref(), Some("session-restored"));
    assert_eq!(state.session.title, "Desktop session title");
    assert_eq!(state.session.scope.as_deref(), Some("project-scope"));
    assert_eq!(runtime.session_id(), Some("session-restored"));

    let mut app = V3GatewayApp::new(runtime, state);
    let rendered = app
        .transcript_lines(80, true)
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("session:     Desktop session title"));
    assert!(rendered.contains("existing question"));
    assert!(rendered.contains("existing answer"));

    let command = app
        .handle_action(V3Action::Submit { text: "continue".to_string(), attachments: Vec::new() })
        .expect("submit restored session");
    let request = app.request_for_command(&command).expect("gateway request");
    assert_eq!(request.session_id.as_ref().map(AsRef::as_ref), Some("session-restored"));
    app.persist_session_snapshot().expect("persist restored session");

    assert_eq!(
        requests.recv().expect("preview request"),
        "/v1/session/ui-previews?directory=%2Fworkspace"
    );
    assert_eq!(requests.recv().expect("scope request"), "/v1/session/scope");
    assert_eq!(requests.recv().expect("snapshot request"), "/v1/session/session-restored/any");
    assert_eq!(
        requests.recv().expect("save request"),
        "/v1/session/session-restored/ui?directory=%2Fworkspace"
    );
    server.join().expect("gateway fixture");
}

fn gateway_fixture(
    responses: Vec<serde_json::Value>,
) -> (GatewayEndpoint, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind gateway fixture");
    let port = listener.local_addr().expect("gateway fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for body in responses {
            let (mut stream, _) = listener.accept().expect("accept gateway request");
            sender.send(read_request_path(&mut stream)).expect("record gateway request");
            let body = body.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write gateway response");
        }
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn read_request_path(stream: &mut TcpStream) -> String {
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
    let path = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("gateway request path")
        .to_string();
    let content_length = headers
        .lines()
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
    path
}
