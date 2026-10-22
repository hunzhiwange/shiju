#![allow(unused_imports, dead_code)]

use std::any::type_name;
use std::fs;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::json;
use vw_gateway_client::{GatewayChatStreamEvent, GatewayClient, GatewayEndpoint};
use vw_tui::{
    ChatWidget, ChatWidgetConfig, TranscriptPager, V3Action, V3ApprovalChoice, V3ApprovalDecision,
    V3ApprovalKind, V3ApprovalRequest, V3ChatStatus, V3DisplayEvent, V3Event, V3FooterData,
    V3UserInputAnswer, V3UserInputOption, V3UserInputQuestion, V3UserInputRequest,
};

use super::app::V3GatewayApp;
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

const SIZES: [(u16, u16); 3] = [(40, 18), (80, 24), (120, 40)];
const SCENARIOS: [&str; 10] = [
    "empty",
    "conversation",
    "working",
    "tools",
    "approval",
    "question",
    "overlay",
    "transcript",
    "completed",
    "error",
];

#[test]
fn tui_real_gateway_pipeline_preserves_v2_semantics() {
    let mut app = gateway_app();

    let submit = app
        .handle_action(V3Action::Submit {
            text: "bridge prompt".to_string(),
            attachments: Vec::new(),
        })
        .expect("TUI submit must route through the controller");
    let request = app
        .request_for_command(&submit)
        .expect("the existing request builder must create the Gateway request");
    assert_eq!(request.session_id.as_ref().map(AsRef::as_ref), Some("session-17"));
    assert_eq!(request.model.as_deref(), Some("openai/gpt-5.6"));
    assert_eq!(
        request.reasoning_effort,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::Medium)
    );
    assert_eq!(request.agent.as_deref(), Some("researcher"));
    assert_eq!(request.acp_agent.as_deref(), Some("codex"));
    assert_eq!(request.allowed_tools, Some(vec!["AgentTool".to_string(), "todowrite".to_string()]));
    assert_eq!(request.acp_allowed_tools, request.allowed_tools);
    assert_eq!(
        request.messages.last().and_then(|message| message["content"].as_str()),
        Some("bridge prompt")
    );
    assert_eq!(request.system.as_ref().map(Vec::len), Some(1));
    let options = request.options.as_ref().expect("Gateway options must be preserved");
    assert_eq!(options["cwd"], "/workspace");
    assert_eq!(options["acp_force_new_session"], true);
    assert_eq!(options["acp_permission_mode"], "approve-reads");
    assert_eq!(options["allowed_tools"], json!(["AgentTool", "todowrite"]));

    assert_eq!(
        app.handle_action(V3Action::Cancel).expect("cancel action"),
        TuiControllerCommand::CancelActiveSubmission
    );
    assert_permission_and_question_commands(&mut app);

    let first = app.accept_gateway_event(GatewayChatStreamEvent::Delta("hello ".to_string()));
    assert!(matches!(first.as_slice(), [V3Event::TurnStarted { .. }, V3Event::TextDelta { .. }]));
    app.accept_gateway_event(GatewayChatStreamEvent::Delta(
        "tool grep\n{\"status\":\"running\",\"title\":\"Search repo\",\"input\":{\"q\":\"needle\"}}"
            .to_string(),
    ));
    app.accept_gateway_event(GatewayChatStreamEvent::Delta(
        "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}".to_string(),
    ));
    app.accept_gateway_event(GatewayChatStreamEvent::Delta("world".to_string()));
    let terminal = app.accept_gateway_event(GatewayChatStreamEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: Some(json!({
            "input_tokens": 11,
            "output_tokens": 7,
            "cached_tokens": 3,
            "reasoning_tokens": 2
        })),
        message_id: Some("assistant-17".to_string()),
        parent_message_id: Some("user-17".to_string()),
    });
    assert!(matches!(terminal.as_slice(), [V3Event::TurnCompleted { .. }]));
    assert_eq!(
        app.state().status.turn_terminal,
        UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) }
    );
    assert!(app.state().messages.iter().any(
        |message| matches!(message, UiMessage::ToolResult(result) if result.content == "2 matches" && !result.is_error)
    ));
    let assistant = app
        .state()
        .messages
        .iter()
        .find_map(|message| match message {
            UiMessage::Assistant(message) => Some(message),
            _ => None,
        })
        .expect("real runtime pipeline must retain the assistant message");
    assert_eq!(assistant.text, "hello world");
    assert_eq!(assistant.usage.input_tokens, 11);
    assert_eq!(assistant.usage.output_tokens, 7);
    assert_eq!(assistant.base.id.as_str(), "gateway:assistant-17");
    assert!(
        app.state()
            .to_chat_session()
            .message_ids
            .iter()
            .flatten()
            .any(|message_id| message_id == "assistant-17")
    );

    for (width, height) in SIZES {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        app.widget().render(area, &mut buffer);
        let rendered = buffer_text(&buffer, area);
        assert!(rendered.contains("hello"));
        assert!(rendered.contains("world"));
        if width == 120 {
            assert!(rendered.contains(">_ ShiJu (v0.4.0)"));
        }
    }

    let mut error_app = gateway_app();
    error_app.accept_gateway_event(GatewayChatStreamEvent::Delta("partial".to_string()));
    let error =
        error_app.accept_gateway_event(GatewayChatStreamEvent::Error("gateway failed".to_string()));
    assert!(
        matches!(error.as_slice(), [V3Event::TurnFailed { message, .. }] if message == "gateway failed")
    );
    assert_eq!(
        error_app.state().status.turn_terminal,
        UiTurnTerminal::Error { message: "gateway failed".to_string() }
    );
}

#[test]
fn tui_forwards_the_initial_display_model_to_gateway_requests() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 9);
    let client = GatewayClient::new(endpoint).expect("test Gateway client");
    let runtime =
        GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")));
    let mut state = TuiState::default();
    state.session_options.model = Some("token-plan/qwen3.7-plus".to_string());
    assert!(state.status.model_name.is_none());

    let mut app = V3GatewayApp::new(runtime, state);
    let submit = app
        .handle_action(V3Action::Submit { text: "who are u".to_string(), attachments: Vec::new() })
        .expect("TUI submit must route through the controller");
    let request = app
        .request_for_command(&submit)
        .expect("the existing request builder must create the Gateway request");

    assert_eq!(request.model.as_deref(), Some("token-plan/qwen3.7-plus"));
}

fn gateway_app() -> V3GatewayApp {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 9);
    let client = GatewayClient::new(endpoint).expect("test Gateway client");
    let runtime = GatewayUiRuntime::new(
        client,
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("session-17".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("session-17".to_string());
    state.project.workspace_root = Some(PathBuf::from("/workspace"));
    state.status.model_name = Some("openai/gpt-5.6".to_string());
    state.session_options.permission_mode = PermissionMode::Plan;
    state.session_options.set_full_access(false);
    state.session_options.set_subagent_mode(true);
    state.session_options.delegate_agent = Some("researcher".to_string());
    state.session_options.select_acp_backend(Some("codex".to_string()));
    V3GatewayApp::new(runtime, state)
}

fn assert_permission_and_question_commands(app: &mut V3GatewayApp) {
    let mut permission_queue = PermissionQueue::default();
    permission_queue.replace(vec![PermissionRequest {
        id: "permission-17".to_string(),
        session_id: "session-17".to_string(),
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
    assert_eq!(
        app.handle_action(V3Action::ResolveApproval {
            request_id: "permission-17".to_string(),
            decision: V3ApprovalDecision::AllowForSession,
        })
        .expect("approval must route to the existing permission command"),
        TuiControllerCommand::Overlay(TuiOverlayCommand::PermissionDecision {
            request_id: "permission-17".to_string(),
            decision: PermissionDecision::Always,
        })
    );

    app.state_mut().tasks.pending_questions.push(UiQuestionOverlay {
        request_id: "question-17".to_string(),
        session_id: "session-17".to_string(),
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
    let command = app
        .handle_action(V3Action::AnswerUserInput {
            request_id: "question-17".to_string(),
            answers: vec![V3UserInputAnswer {
                question_id: "0".to_string(),
                values: vec!["lib".to_string()],
            }],
        })
        .expect("question must route to the existing question command");
    let TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionSubmitted(question)) = command
    else {
        panic!("expected existing question submission command");
    };
    assert_eq!(question.request_id, "question-17");
    assert_eq!(question.answers, vec![vec!["lib".to_string()]]);
}

fn fixture_bundle(scenario: &str) -> String {
    SIZES
        .into_iter()
        .map(|(width, height)| {
            format!(
                "=== {width}x{height} ===\n{}",
                render_scenario(scenario, Rect::new(0, 0, width, height))
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fixture_path(scenario: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tui_codex_port")
        .join(format!("{scenario}.frame"))
}

fn render_scenario(scenario: &str, area: Rect) -> String {
    let widget = visual_widget(scenario);
    let mut buffer = Buffer::empty(area);
    if scenario == "transcript" {
        let mut pager = TranscriptPager::new(widget.transcript_lines(area.width, true));
        pager.scroll_to_top();
        pager.render(area, &mut buffer);
    } else {
        widget.render(area, &mut buffer);
        if let Some((x, y)) = widget.cursor_pos(area) {
            assert!(area.contains((x, y).into()), "cursor left {scenario} frame at {area:?}");
        }
    }
    buffer_text(&buffer, area)
}

fn buffer_text(buffer: &Buffer, area: Rect) -> String {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn visual_widget(scenario: &str) -> ChatWidget {
    let mut widget = ChatWidget::new(ChatWidgetConfig {
        footer_data: V3FooterData {
            model: Some("gpt-5.6".to_string()),
            permission: Some("workspace write".to_string()),
            directory: Some("/workspace/vibe-window".to_string()),
            context_window_percent: Some(42),
            context_window_used_tokens: Some(12_345),
            is_task_running: false,
        },
        placeholder: "Describe the next task".to_string(),
    });

    match scenario {
        "empty" => {}
        "conversation" => push_conversation(&mut widget),
        "working" => {
            push_conversation(&mut widget);
            widget.set_status(V3ChatStatus::running(
                "Working",
                Some("Reading copied components".to_string()),
            ));
            widget.handle_v3_event(V3Event::TurnStarted { turn_id: "turn-working".to_string() });
            widget.handle_v3_event(V3Event::ThinkingDelta {
                turn_id: "turn-working".to_string(),
                delta: "Inspecting the Gateway bridge and terminal buffer.".to_string(),
            });
            widget.handle_v3_event(V3Event::TextDelta {
                turn_id: "turn-working".to_string(),
                delta: "The matrix is still rendering".to_string(),
            });
        }
        "tools" => {
            widget.handle_v3_event(V3Event::TurnStarted { turn_id: "turn-tools".to_string() });
            widget.handle_v3_event(V3Event::ToolStarted {
                turn_id: "turn-tools".to_string(),
                call_id: "call-17".to_string(),
                tool: "shell".to_string(),
                command: "cargo test -p vw-cli".to_string(),
            });
            widget.handle_v3_event(V3Event::ToolOutputDelta {
                turn_id: "turn-tools".to_string(),
                call_id: "call-17".to_string(),
                delta: "running 2 tests\n".to_string(),
            });
            widget.handle_v3_event(V3Event::ToolCompleted {
                turn_id: "turn-tools".to_string(),
                call_id: "call-17".to_string(),
                output: "2 passed; 0 failed".to_string(),
                exit_code: 0,
                duration_ms: 87,
            });
        }
        "approval" => {
            push_conversation(&mut widget);
            widget.show_approval(approval_request(V3ApprovalKind::Command, "Run focused tests?"));
        }
        "question" => {
            push_conversation(&mut widget);
            widget.show_user_input(V3UserInputRequest::new(
                "question-matrix",
                vec![V3UserInputQuestion::new(
                    "target",
                    "Target",
                    "Which validation target should run?",
                    vec![
                        V3UserInputOption::new("Library", "Run the focused library target"),
                        V3UserInputOption::new("Bridge", "Run the Gateway bridge target"),
                    ],
                )],
            ));
        }
        "overlay" => {
            push_conversation(&mut widget);
            widget.show_approval(approval_request(
                V3ApprovalKind::Network,
                "Allow local Gateway connection?",
            ));
        }
        "transcript" => {
            for index in 0..18 {
                widget.push_history(V3DisplayEvent::User {
                    text: format!("Prompt {index:02}: verify copied terminal layout"),
                    attachments: Vec::new(),
                });
                widget.push_history(V3DisplayEvent::Assistant {
                    markdown: format!("Response {index:02}: upstream component path retained."),
                });
            }
        }
        "completed" => {
            widget.handle_v3_event(V3Event::TurnStarted { turn_id: "turn-completed".to_string() });
            widget.handle_v3_event(V3Event::TextDelta {
                turn_id: "turn-completed".to_string(),
                delta: "Gateway regression completed successfully.".to_string(),
            });
            widget
                .handle_v3_event(V3Event::TurnCompleted { turn_id: "turn-completed".to_string() });
        }
        "error" => {
            widget.handle_v3_event(V3Event::TurnStarted { turn_id: "turn-error".to_string() });
            widget.handle_v3_event(V3Event::ThinkingDelta {
                turn_id: "turn-error".to_string(),
                delta: "Checking the final terminal event.".to_string(),
            });
            widget.handle_v3_event(V3Event::TurnFailed {
                turn_id: "turn-error".to_string(),
                message: "Gateway stream failed".to_string(),
            });
        }
        _ => panic!("unknown visual scenario: {scenario}"),
    }
    widget
}

fn push_conversation(widget: &mut ChatWidget) {
    widget.push_history(V3DisplayEvent::User {
        text: "Show the final copied TUI matrix.".to_string(),
        attachments: Vec::new(),
    });
    widget.push_history(V3DisplayEvent::Assistant {
        markdown: "The **Codex-derived** ChatWidget is connected to ShiJu Gateway."
            .to_string(),
    });
}

fn approval_request(kind: V3ApprovalKind, title: &str) -> V3ApprovalRequest {
    V3ApprovalRequest::new(
        "approval-matrix",
        kind,
        title,
        "cargo test -p vw-cli tui --lib",
        vec![
            V3ApprovalChoice::new("allow", "Allow once", V3ApprovalDecision::Allow),
            V3ApprovalChoice::new(
                "session",
                "Allow for session",
                V3ApprovalDecision::AllowForSession,
            ),
            V3ApprovalChoice::new("deny", "Deny", V3ApprovalDecision::Deny),
        ],
    )
}
