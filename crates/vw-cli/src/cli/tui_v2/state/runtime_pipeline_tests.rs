use vw_gateway_client::GatewayChatUsage;

use super::{apply_runtime_event, apply_runtime_event_at, parse_tool_delta};
use crate::cli::tui_v2::model::{UiMessage, UiThinkingPhase, UiToolCallState, UiTurnTerminal};
use crate::cli::tui_v2::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use crate::cli::tui_v2::state::TuiState;

fn usage() -> GatewayChatUsage {
    GatewayChatUsage { input_tokens: 10, output_tokens: 20, cached_tokens: 3, reasoning_tokens: 4 }
}

#[test]
fn tui_thinking_stream_transitions_match_reference() {
    let mut state = TuiState::default();

    apply_runtime_event_at(&mut state, UiRuntimeEvent::Delta("<think>inspect".to_string()), 1_000);
    let (thinking_id, projection_before_delta) = match state.messages.as_slice() {
        [UiMessage::Thinking(block)] => {
            assert_eq!(block.phase, UiThinkingPhase::Streaming);
            assert_eq!(block.timing[0].start_ms, 1_000);
            assert_eq!(block.timing[0].last_update_ms, 1_000);
            (block.base.id.clone(), state.transcript.clone())
        }
        messages => panic!("expected one streaming thinking block, got {messages:?}"),
    };

    apply_runtime_event_at(&mut state, UiRuntimeEvent::Delta(" reducer".to_string()), 1_250);
    let UiMessage::Thinking(streaming) = &state.messages[0] else {
        panic!("thinking identity must stay stable");
    };
    assert_eq!(streaming.base.id, thinking_id);
    assert_eq!(streaming.content, "inspect reducer");
    assert_eq!(streaming.timing[0].last_update_ms, 1_250);
    assert_eq!(
        state.transcript, projection_before_delta,
        "a thinking tick/delta must not rebuild the transcript projection"
    );

    apply_runtime_event_at(
        &mut state,
        UiRuntimeEvent::Delta("</think>\nanswer".to_string()),
        1_500,
    );
    let UiMessage::Thinking(closed) = &state.messages[0] else {
        panic!("closed thinking block must keep its original identity");
    };
    assert_eq!(closed.base.id, thinking_id);
    assert_eq!(closed.phase, UiThinkingPhase::Closed);
    assert_eq!(closed.timing[0].end_ms, Some(1_500));
    assert!(closed.collapsed);
    assert!(matches!(
        &state.messages[1],
        UiMessage::Assistant(message) if message.text == "answer"
    ));

    apply_runtime_event_at(
        &mut state,
        UiRuntimeEvent::Delta(
            "<redacted-thinking>never expose this payload</redacted-thinking>".to_string(),
        ),
        2_000,
    );
    let redacted = state
        .messages
        .iter()
        .find_map(|message| match message {
            UiMessage::Thinking(block) if block.phase == UiThinkingPhase::Redacted => Some(block),
            _ => None,
        })
        .expect("redacted thinking placeholder");
    assert!(redacted.content.is_empty());
    assert!(
        !format!("{:?}", state.messages).contains("never expose this payload"),
        "redacted thinking payload must never enter UI state"
    );
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
fn thinking_alias_tags_split_delta_like_think_tags() {
    let mut state = TuiState::default();

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta("visible <thinking>\nplan\n</thinking>\nafter".to_string()),
    );

    assert!(!state.runtime.thinking_open);
    assert!(state.messages.iter().any(|message| {
        matches!(message, UiMessage::Thinking(block) if block.content.contains("plan"))
    }));
    assert!(state.messages.iter().any(|message| {
        matches!(message, UiMessage::Assistant(assistant) if assistant.text == "visible after")
    }));
}

#[test]
fn nested_thinking_aliases_do_not_leak_transport_tags() {
    let mut state = TuiState::default();

    for delta in [
        "<think>",
        "<thinking>The file is unchanged.</thinking>Now edit it.</think>",
        "answer</thinking>",
    ] {
        apply_runtime_event(&mut state, UiRuntimeEvent::Delta(delta.to_string()));
    }

    assert!(!state.runtime.thinking_open);
    let thinking = state
        .messages
        .iter()
        .filter_map(|message| match message {
            UiMessage::Thinking(block) => Some(block.content.as_str()),
            _ => None,
        })
        .collect::<String>();
    let answer = state
        .messages
        .iter()
        .filter_map(|message| match message {
            UiMessage::Assistant(message) => Some(message.text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(thinking, "The file is unchanged.Now edit it.");
    assert_eq!(answer, "answer");
    assert!(!thinking.contains("<think"));
    assert!(!answer.contains("</think"));
}

#[test]
fn reasoning_marker_creates_closed_thinking_block() {
    let mut state = TuiState::default();

    apply_runtime_event_at(
        &mut state,
        UiRuntimeEvent::Delta("answer\n\n[reasoning]\nchain".to_string()),
        1_000,
    );

    assert!(matches!(
        &state.messages[0],
        UiMessage::Assistant(message) if message.text == "answer"
    ));
    assert!(matches!(
        &state.messages[1],
        UiMessage::Thinking(block)
            if block.content == "chain"
                && block.phase == UiThinkingPhase::Closed
                && block.collapsed
    ));
}

#[test]
fn inline_reasoning_text_remains_assistant_content() {
    let mut state = TuiState::default();
    let delta = "answer [reasoning] is a label";

    apply_runtime_event(&mut state, UiRuntimeEvent::Delta(delta.to_string()));

    assert!(matches!(
        &state.messages[0],
        UiMessage::Assistant(message) if message.text == delta
    ));
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
fn tool_input_json_deltas_append_without_duplicating_the_call() {
    let mut state = TuiState::default();
    for delta in [
        r#"tool read
{"status":"queued","title":"Read file"}"#,
        r#"tool read
{"status":"input_json_delta","input_json_delta":"{\"path\":"}"#,
        r#"tool read
{"status":"input_json_delta","input_json_delta":"\"README.md\"}"}"#,
        r#"tool read
{"status":"completed","output":"contents"}"#,
    ] {
        apply_runtime_event(&mut state, UiRuntimeEvent::Delta(delta.to_string()));
    }

    let calls = state
        .messages
        .iter()
        .filter_map(|message| match message {
            UiMessage::ToolCall(call) => Some(call),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments.as_deref(), Some(r#"{"path":"README.md"}"#));
    assert_eq!(calls[0].state, UiToolCallState::Complete);

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::Delta(
            r#"tool shell
{"status":"cancelled","error":"interrupted by user"}"#
                .to_string(),
        ),
    );
    assert!(state.messages.iter().any(
        |message| matches!(message, UiMessage::ToolCall(call) if call.state == UiToolCallState::Cancelled)
    ));
}

#[test]
fn tool_delta_parser_rejects_invalid_headers_payloads_and_statuses() {
    assert!(parse_tool_delta("tool grep").is_none());
    assert!(parse_tool_delta("grep\n{\"status\":\"running\"}").is_none());
    assert!(parse_tool_delta("tool \n{\"status\":\"running\"}").is_none());
    assert!(parse_tool_delta("tool grep\nnot json").is_none());
    assert!(parse_tool_delta("tool grep\n{\"status\":\"\"}").is_none());
    assert!(parse_tool_delta("tool grep\n{\"status\":\"paused\"}").is_none());
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
    apply_runtime_event(&mut state, UiRuntimeEvent::PostToolRound);

    assert_eq!(state.session.title, "Stable");
    assert!(state.messages.is_empty());
    assert_eq!(state.status.turn_terminal, UiTurnTerminal::Pending);
}

#[test]
fn context_loaded_event_adds_informational_message() {
    let mut state = TuiState::default();

    apply_runtime_event(
        &mut state,
        UiRuntimeEvent::ContextLoaded {
            paths: vec!["/work/AGENTS.md".to_string(), "/work/CONTEXT.md".to_string()],
        },
    );

    assert!(state.messages.iter().any(|message| matches!(
        message,
        UiMessage::System(system)
            if system.text == "已加载上下文文件：\n- /work/AGENTS.md\n- /work/CONTEXT.md"
    )));
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
