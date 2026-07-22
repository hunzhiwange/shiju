use vw_gateway_client::GatewayChatUsage;

use super::{apply_runtime_event, parse_tool_delta};
use crate::cli::tui_v2::model::{UiMessage, UiToolCallState, UiTurnTerminal};
use crate::cli::tui_v2::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use crate::cli::tui_v2::state::TuiState;

fn usage() -> GatewayChatUsage {
    GatewayChatUsage { input_tokens: 10, output_tokens: 20, cached_tokens: 3, reasoning_tokens: 4 }
}

#[test]
fn thinking_tags_split_delta_and_close_before_terminal() {
    let mut state = TuiState::default();

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta("visible <think>\nplan\n</think>\nafter".to_string()),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: Some("assistant-id".to_string()),
            parent_message_id: None,
        }),
    );

    assert!(!state.runtime.thinking_open);
    assert_eq!(
        state.status.turn_terminal,
        UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) }
    );
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::Thinking(block) if block.content.contains("plan") && block.timing.last().and_then(|timing| timing.end_ms).is_some())));
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::Assistant(assistant) if assistant.text == "visible after" && assistant.base.id.as_str() == "gateway:assistant-id")));
}

#[test]
fn empty_thinking_block_can_span_events_without_emitting_empty_message() {
    let mut state = TuiState::default();

    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("<think>".to_string()));

    assert!(state.runtime.thinking_open);
    assert!(state.messages.is_empty());

    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("</think>\nafter".to_string()));

    assert!(!state.runtime.thinking_open);
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(
        &state.messages[0],
        UiMessage::Assistant(assistant) if assistant.text == "after"
    ));
}

#[test]
fn open_thinking_block_is_closed_by_error_terminal_and_side_message_is_added() {
    let mut state = TuiState::default();

    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("<think>partial".to_string()));
    assert!(state.runtime.thinking_open);

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error("gateway failed".to_string())),
    );

    assert!(!state.runtime.thinking_open);
    assert_eq!(
        state.status.turn_terminal,
        UiTurnTerminal::Error { message: "gateway failed".to_string() }
    );
    assert!(state.messages.iter().any(
        |message| matches!(message, UiMessage::Error(error) if error.message == "gateway failed")
    ));
}

#[test]
fn tool_delta_creates_call_and_result_messages_for_success_and_failure() {
    let mut state = TuiState::default();

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta(
            "tool grep\n{\"status\":\"running\",\"title\":\"Search repo\",\"input\":{\"q\":\"needle\"}}"
                .to_string(),
        ),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta(
            "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}".to_string(),
        ),
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta("tool write\n{\"status\":\"denied\"}".to_string()),
    );

    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::ToolCall(call) if call.tool_name == "grep" && call.summary.as_deref() == Some("Search repo") && call.arguments.as_deref() == Some("{\"q\":\"needle\"}") && call.state == UiToolCallState::Complete)));
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::ToolResult(result) if result.tool_name == "grep" && result.content == "2 matches" && !result.is_error)));
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::ToolResult(result) if result.tool_name == "write" && result.content == "tool write failed" && result.is_error)));
}

#[test]
fn tool_delta_parser_rejects_invalid_headers_payloads_and_statuses() {
    assert!(parse_tool_delta("tool grep").is_none());
    assert!(parse_tool_delta("grep\n{\"status\":\"running\"}").is_none());
    assert!(parse_tool_delta("tool \n{\"status\":\"running\"}").is_none());
    assert!(parse_tool_delta("tool grep\nnot json").is_none());
    assert!(parse_tool_delta("tool grep\n{\"status\":\"\"}").is_none());
    assert!(parse_tool_delta("tool grep\n{\"status\":\"queued\"}").is_none());
}

#[test]
fn tool_delta_parser_normalizes_optional_text_and_failure_payloads() {
    let running = parse_tool_delta(
        "tool shell\n{\"status\":\"running\",\"title\":\"shell\",\"input\":\"  cargo check  \"}",
    )
    .expect("running tool delta should parse");

    assert_eq!(running.tool_name, "shell");
    assert_eq!(running.summary, None);
    assert_eq!(running.arguments.as_deref(), Some("cargo check"));
    assert_eq!(running.state, UiToolCallState::Running);
    assert!(running.result.is_none());

    let completed = parse_tool_delta(
        "tool json\n{\"status\":\"completed\",\"title\":\"Json Tool\",\"input\":{\"q\":\"needle\"},\"output\":{\"matches\":2}}",
    )
    .expect("completed tool delta should parse");

    assert_eq!(completed.summary.as_deref(), Some("Json Tool"));
    assert_eq!(completed.arguments.as_deref(), Some("{\"q\":\"needle\"}"));
    assert_eq!(completed.state, UiToolCallState::Complete);
    let completed_result = completed.result.expect("completed tool should have result");
    assert_eq!(completed_result.content, "{\"matches\":2}");
    assert!(!completed_result.is_error);

    let failed = parse_tool_delta(
        "tool shell\n{\"status\":\"failed\",\"error\":\"  permission denied  \",\"output\":\"fallback\"}",
    )
    .expect("failed tool delta should parse");

    assert_eq!(failed.state, UiToolCallState::Failed);
    let failed_result = failed.result.expect("failed tool should have result");
    assert_eq!(failed_result.content, "permission denied");
    assert!(failed_result.is_error);
}

#[test]
fn step_and_timeout_terminal_update_state_and_usage() {
    let mut state = TuiState::default();

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::StepStart {
            step_index: 3,
            created_ms: 40,
            model: Some("model-a".to_string()),
        },
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::StepFinish {
            step_index: 3,
            finished_ms: 50,
            usage: usage(),
            finish_reason: Some("stop".to_string()),
            model: Some("model-b".to_string()),
        },
    );
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::TimedOut {
            message: "deadline exceeded".to_string(),
            usage: Some(usage()),
            message_id: Some("assistant-final".to_string()),
            parent_message_id: Some("user-parent".to_string()),
        }),
    );

    assert_eq!(
        state.status.turn_terminal,
        UiTurnTerminal::TimedOut { message: "deadline exceeded".to_string() }
    );
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::Step(step) if step.step_index == 3 && step.finished_ms == Some(50) && step.usage.output_tokens == 20 && step.model.as_deref() == Some("model-b"))));
    assert!(state.messages.iter().any(
        |message| matches!(message, UiMessage::Error(error) if error.message == "deadline exceeded")
    ));
}

#[test]
fn cancelled_terminal_updates_assistant_without_side_error_message() {
    let mut state = TuiState::default();

    apply_runtime_event(&mut state, UiRuntimeEvent::Delta("partial answer".to_string()));
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Cancelled {
            reason: Some("user cancelled".to_string()),
            usage: Some(usage()),
            message_id: Some("assistant-cancelled".to_string()),
            parent_message_id: Some("user-parent".to_string()),
        }),
    );

    assert_eq!(
        state.status.turn_terminal,
        UiTurnTerminal::Cancelled { reason: Some("user cancelled".to_string()) }
    );
    assert!(state.messages.iter().all(|message| !matches!(message, UiMessage::Error(_))));
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::Assistant(assistant) if assistant.text == "partial answer" && assistant.usage.input_tokens == 10 && assistant.base.id.as_str() == "gateway:assistant-cancelled" && assistant.base.parent_id.as_ref().map(|id| id.as_str()) == Some("gateway:user-parent"))));
}

#[test]
fn ignored_runtime_events_do_not_mutate_visible_state() {
    let mut state = TuiState::default();
    state.session.title = "Stable".to_string();

    apply_runtime_event(&mut state, UiRuntimeEvent::TaskStateChanged { session_id: None });
    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::UsageUpdated { session_id: Some("session".to_string()), usage: usage() },
    );

    assert_eq!(state.session.title, "Stable");
    assert!(state.messages.is_empty());
    assert_eq!(state.status.turn_terminal, UiTurnTerminal::Pending);
}

#[test]
fn metadata_and_unknown_events_update_only_expected_state() {
    let mut state = TuiState::default();
    state.session.session_id = Some("current".to_string());
    state.session.title = "Old".to_string();
    state.refresh_session_preview();

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::SessionMetadataChanged {
            session_id: Some("other".to_string()),
            title: Some("Ignored".to_string()),
        },
    );
    assert_eq!(state.session.title, "Old");

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::SessionMetadataChanged {
            session_id: Some("current".to_string()),
            title: Some("Renamed".to_string()),
        },
    );
    assert_eq!(state.session.title, "Renamed");

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::SessionMetadataChanged {
            session_id: Some("current".to_string()),
            title: None,
        },
    );
    assert_eq!(state.session.title, "Renamed");

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::SessionMetadataChanged {
            session_id: Some("   ".to_string()),
            title: Some("Blank target".to_string()),
        },
    );
    assert_eq!(state.session.title, "Blank target");

    apply_runtime_event(&mut state, UiRuntimeEvent::Unknown { event_type: None });
    assert!(state.messages.iter().any(|message| matches!(message, UiMessage::System(system) if system.text == "Unsupported gateway event: unknown.")));
}
