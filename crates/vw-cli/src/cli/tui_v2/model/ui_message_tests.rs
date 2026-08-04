use vw_shared::session::ui_types as session_ui;
use vw_gateway_client::GatewayChatStreamEvent;

use super::attachment::{UiAttachment, UiAttachmentKind};
use super::ui_message::{
    UiAssistantMessage, UiCompactBoundary, UiErrorMessage, UiMessage, UiMessageBase, UiMessageId,
    UiMessageKind, UiMessageOrigin, UiProgressMessage, UiStep, UiStepState, UiSystemMessage,
    UiSystemMessageLevel, UiThinkingBlock, UiThinkingPhase, UiThinkingTiming, UiTokenUsage, UiToolCall,
    UiToolCallState, UiToolResult, UiTurnTerminal, UiUnknownMessage, UiUserMessage,
};
use crate::cli::tui_v2::runtime::stream_adapter::adapt_gateway_stream_event;
use crate::cli::tui_v2::state::{TuiState, apply_runtime_event};

#[test]
fn tui_message_model_roundtrips_reference_content_kinds() {
    let mut state = TuiState::default();
    state.session.session_id = Some("session-content".to_string());

    let events = [
        GatewayChatStreamEvent::Other(serde_json::json!({
            "type": "chat.content",
            "message_id": "attachment-1",
            "parent_message_id": "assistant-1",
            "created_ms": 10,
            "content": {
                "type": "attachment",
                "kind": "image",
                "name": "diagram.png",
                "media_type": "image/png",
                "uri": "file:///tmp/diagram.png",
                "alt_text": "architecture diagram"
            }
        })),
        GatewayChatStreamEvent::Other(serde_json::json!({
            "type": "chat.content",
            "message_id": "progress-1",
            "parent_message_id": "assistant-1",
            "created_ms": 20,
            "content": {
                "type": "progress",
                "label": "Indexing",
                "current": 3,
                "total": 8,
                "completed": false
            }
        })),
        GatewayChatStreamEvent::Other(serde_json::json!({
            "type": "chat.content",
            "message_id": "compact-1",
            "parent_message_id": "assistant-1",
            "created_ms": 30,
            "origin": {"type": "command", "name": "compact"},
            "content": {
                "type": "compact_boundary",
                "summary": "Earlier context compacted",
                "compacted_messages": 12
            }
        })),
        GatewayChatStreamEvent::Other(serde_json::json!({
            "type": "chat.content",
            "message_id": "unknown-1",
            "parent_message_id": "assistant-1",
            "created_ms": 40,
            "content": {
                "type": "future_secret_block",
                "api_token": "must-not-be-persisted"
            }
        })),
    ];

    for event in events {
        apply_runtime_event(&mut state, adapt_gateway_stream_event(event));
    }

    assert!(matches!(
        &state.messages[0],
        UiMessage::Attachment(UiAttachment {
            kind: UiAttachmentKind::Image,
            name,
            media_type: Some(media_type),
            uri,
            alt_text: Some(alt_text),
            ..
        }) if name == "diagram.png"
            && media_type == "image/png"
            && uri == "file:///tmp/diagram.png"
            && alt_text == "architecture diagram"
    ));
    assert!(matches!(
        &state.messages[1],
        UiMessage::Progress(UiProgressMessage {
            label,
            current: Some(3),
            total: Some(8),
            completed: false,
            ..
        }) if label == "Indexing"
    ));
    assert!(matches!(
        &state.messages[2],
        UiMessage::CompactBoundary(UiCompactBoundary {
            summary,
            compacted_messages: 12,
            ..
        }) if summary == "Earlier context compacted"
    ));
    assert!(matches!(
        state.messages[2].base().origin.as_ref(),
        Some(UiMessageOrigin::Command { name }) if name == "compact"
    ));
    assert!(matches!(
        &state.messages[3],
        UiMessage::Unknown(UiUnknownMessage { external_kind, summary, .. })
            if external_kind == "future_secret_block"
                && summary.contains("Unsupported content block")
    ));

    let snapshot = state.to_chat_session();
    assert_eq!(snapshot.messages.len(), 4);
    assert!(
        snapshot
            .messages
            .iter()
            .all(|message| !message.content.contains("must-not-be-persisted"))
    );

    let restored = TuiState::from_chat_session(&snapshot);
    assert_eq!(restored.messages, state.messages);
    assert!(restored.search_index.is_aligned(restored.messages.len()));
}

#[test]
fn ui_message_id_constructors_preserve_namespaces_and_raw_values() {
    assert_eq!(UiMessageId::new("raw-1").as_str(), "raw-1");
    assert_eq!(UiMessageId::local("draft").as_str(), "local:draft");
    assert_eq!(UiMessageId::gateway("msg").as_str(), "gateway:msg");
}

#[test]
fn ui_message_base_builders_attach_optional_metadata() {
    let base = UiMessageBase::new(UiMessageId::new("child"))
        .with_parent_id(UiMessageId::new("parent"))
        .with_session_id("session-1")
        .with_created_ms(123);

    assert_eq!(base.id.as_str(), "child");
    assert_eq!(base.parent_id.as_ref().map(UiMessageId::as_str), Some("parent"));
    assert_eq!(base.session_id.as_deref(), Some("session-1"));
    assert_eq!(base.created_ms, Some(123));
}

#[test]
fn token_usage_and_thinking_timing_convert_from_shared_owned_and_borrowed_values() {
    let usage = session_ui::TokenUsage {
        input_tokens: 1,
        output_tokens: 2,
        cached_tokens: 3,
        reasoning_tokens: 4,
    };

    assert_eq!(
        UiTokenUsage::from(&usage),
        UiTokenUsage { input_tokens: 1, output_tokens: 2, cached_tokens: 3, reasoning_tokens: 4 }
    );
    assert_eq!(UiTokenUsage::from(usage).reasoning_tokens, 4);

    let timing = session_ui::ThinkTiming { start_ms: 10, end_ms: Some(20), last_update_ms: 30 };
    assert_eq!(
        UiThinkingTiming::from(&timing),
        UiThinkingTiming { start_ms: 10, end_ms: Some(20), last_update_ms: 30 }
    );
    assert_eq!(UiThinkingTiming::from(timing).last_update_ms, 30);
}

#[test]
fn ui_message_kind_and_base_cover_every_variant() {
    let cases = vec![
        (
            UiMessage::User(UiUserMessage {
                base: UiMessageBase::new(UiMessageId::new("user")),
                text: "hello".to_string(),
            }),
            UiMessageKind::User,
        ),
        (
            UiMessage::Assistant(UiAssistantMessage {
                base: UiMessageBase::new(UiMessageId::new("assistant")),
                text: "answer".to_string(),
                usage: UiTokenUsage::default(),
                step_count: 0,
                terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
                model: Some("gpt".to_string()),
            }),
            UiMessageKind::Assistant,
        ),
        (
            UiMessage::ToolCall(UiToolCall {
                base: UiMessageBase::new(UiMessageId::new("call")),
                call_id: Some("call-1".to_string()),
                tool_name: "read".to_string(),
                summary: None,
                arguments: Some("{}".to_string()),
                state: UiToolCallState::Running,
            }),
            UiMessageKind::ToolCall,
        ),
        (
            UiMessage::ToolResult(UiToolResult {
                base: UiMessageBase::new(UiMessageId::new("result")),
                call_id: Some("call-1".to_string()),
                tool_name: "read".to_string(),
                content: "ok".to_string(),
                is_error: false,
            }),
            UiMessageKind::ToolResult,
        ),
        (
            UiMessage::Thinking(UiThinkingBlock {
                base: UiMessageBase::new(UiMessageId::new("thinking")),
                summary: Some("plan".to_string()),
                content: "details".to_string(),
                timing: Vec::new(),
                collapsed: false,
                phase: UiThinkingPhase::default(),
            }),
            UiMessageKind::Thinking,
        ),
        (
            UiMessage::Step(UiStep {
                base: UiMessageBase::new(UiMessageId::new("step")),
                step_index: 1,
                started_ms: 10,
                finished_ms: None,
                usage: UiTokenUsage::default(),
                finish_reason: None,
                model: None,
                state: UiStepState::Pending,
            }),
            UiMessageKind::Step,
        ),
        (
            UiMessage::System(UiSystemMessage {
                base: UiMessageBase::new(UiMessageId::new("system")),
                text: "note".to_string(),
                level: UiSystemMessageLevel::Warning,
            }),
            UiMessageKind::System,
        ),
        (
            UiMessage::Error(UiErrorMessage {
                base: UiMessageBase::new(UiMessageId::new("error")),
                message: "boom".to_string(),
                recoverable: true,
            }),
            UiMessageKind::Error,
        ),
    ];

    for (message, kind) in cases {
        assert_eq!(message.kind(), kind);
        assert_eq!(message.id(), &message.base().id);
    }
}

#[test]
fn default_terminal_and_execution_states_match_initial_lifecycle() {
    assert_eq!(UiTurnTerminal::default(), UiTurnTerminal::Pending);
    assert_eq!(UiToolCallState::default(), UiToolCallState::Queued);
    assert_eq!(UiStepState::default(), UiStepState::Pending);
    assert_eq!(UiSystemMessageLevel::default(), UiSystemMessageLevel::Info);
}

#[test]
fn ui_turn_terminal_all_variants_have_expected_data() {
    assert_eq!(UiTurnTerminal::Pending, UiTurnTerminal::Pending);
    assert_eq!(UiTurnTerminal::Streaming, UiTurnTerminal::Streaming);

    let done = UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) };
    assert_eq!(done, UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) });
    assert_ne!(done, UiTurnTerminal::Done { finish_reason: None });

    let cancelled = UiTurnTerminal::Cancelled { reason: Some("user".to_string()) };
    assert_eq!(cancelled, UiTurnTerminal::Cancelled { reason: Some("user".to_string()) });

    let timed_out = UiTurnTerminal::TimedOut { message: "timeout".to_string() };
    assert_eq!(timed_out, UiTurnTerminal::TimedOut { message: "timeout".to_string() });
    assert_ne!(timed_out, UiTurnTerminal::TimedOut { message: "other".to_string() });

    let error = UiTurnTerminal::Error { message: "err".to_string() };
    assert_eq!(error, UiTurnTerminal::Error { message: "err".to_string() });
    assert_ne!(error, UiTurnTerminal::Error { message: "diff".to_string() });
}

#[test]
fn ui_tool_call_state_all_variants_round_trip() {
    assert_eq!(UiToolCallState::Queued, UiToolCallState::Queued);
    assert_eq!(UiToolCallState::Running, UiToolCallState::Running);
    assert_eq!(UiToolCallState::Complete, UiToolCallState::Complete);
    assert_eq!(UiToolCallState::Failed, UiToolCallState::Failed);
    assert_ne!(UiToolCallState::Queued, UiToolCallState::Complete);
}

#[test]
fn ui_step_state_all_variants_round_trip() {
    assert_eq!(UiStepState::Pending, UiStepState::Pending);
    assert_eq!(UiStepState::Running, UiStepState::Running);
    assert_eq!(UiStepState::Complete, UiStepState::Complete);
    assert_eq!(UiStepState::Cancelled, UiStepState::Cancelled);
    assert_eq!(UiStepState::Failed, UiStepState::Failed);
    assert_ne!(UiStepState::Pending, UiStepState::Running);
}

#[test]
fn ui_system_message_level_all_variants_have_disparate_values() {
    assert_eq!(UiSystemMessageLevel::Info, UiSystemMessageLevel::Info);
    assert_eq!(UiSystemMessageLevel::Warning, UiSystemMessageLevel::Warning);
    assert_eq!(UiSystemMessageLevel::Success, UiSystemMessageLevel::Success);
    assert_ne!(UiSystemMessageLevel::Info, UiSystemMessageLevel::Warning);
    assert_ne!(UiSystemMessageLevel::Info, UiSystemMessageLevel::Success);
}

#[test]
fn ui_message_id_edge_cases_round_trip_correctly() {
    // Empty string preservation
    let empty = UiMessageId::new("");
    assert_eq!(empty.as_str(), "");

    // Special characters preserved through namespace constructors
    assert_eq!(UiMessageId::new("special_chars!@#").as_str(), "special_chars!@#");
    assert_eq!(UiMessageId::local("").as_str(), "local:");
    assert_eq!(UiMessageId::gateway("").as_str(), "gateway:");

    // Unicode preserved
    let unicode = UiMessageId::new("消息-123");
    assert_eq!(unicode.as_str(), "消息-123");
    assert_eq!(UiMessageId::local("消息").as_str(), "local:消息");

    // UiMessageId equality
    assert_eq!(UiMessageId::new("a"), UiMessageId::new("a"));
    assert_ne!(UiMessageId::new("a"), UiMessageId::new("b"));
}

#[test]
fn ui_message_base_with_only_id_has_default_optionals() {
    let minimal = UiMessageBase::new(UiMessageId::new("minimal"));
    assert_eq!(minimal.id.as_str(), "minimal");
    assert!(minimal.parent_id.is_none());
    assert!(minimal.session_id.is_none());
    assert!(minimal.created_ms.is_none());
}

#[test]
fn ui_message_base_builders_chain_in_arbitrary_order() {
    // parent_id before session_id
    let a = UiMessageBase::new(UiMessageId::new("a"))
        .with_parent_id(UiMessageId::new("parent"))
        .with_session_id("session");
    assert_eq!(a.parent_id.as_ref().map(UiMessageId::as_str), Some("parent"));
    assert_eq!(a.session_id.as_deref(), Some("session"));

    // created_ms before parent_id
    let b = UiMessageBase::new(UiMessageId::new("b"))
        .with_created_ms(999)
        .with_parent_id(UiMessageId::new("parent"));
    assert_eq!(b.created_ms, Some(999));
    assert_eq!(b.parent_id.as_ref().map(UiMessageId::as_str), Some("parent"));

    // All four fields set (id is always first)
    let c = UiMessageBase::new(UiMessageId::new("c"))
        .with_session_id("sess")
        .with_created_ms(1)
        .with_parent_id(UiMessageId::new("p"));
    assert_eq!(c.id.as_str(), "c");
    assert_eq!(c.session_id.as_deref(), Some("sess"));
    assert_eq!(c.created_ms, Some(1));
    assert_eq!(c.parent_id.as_ref().map(UiMessageId::as_str), Some("p"));
}

#[test]
fn ui_message_kind_matches_each_variant_individually() {
    let user = UiMessage::User(UiUserMessage {
        base: UiMessageBase::new(UiMessageId::new("u")),
        text: String::new(),
    });
    assert_eq!(user.kind(), UiMessageKind::User);

    let assistant = UiMessage::Assistant(UiAssistantMessage {
        base: UiMessageBase::new(UiMessageId::new("a")),
        text: String::new(),
        usage: UiTokenUsage::default(),
        step_count: 0,
        terminal: UiTurnTerminal::default(),
        model: None,
    });
    assert_eq!(assistant.kind(), UiMessageKind::Assistant);

    let tool_call = UiMessage::ToolCall(UiToolCall {
        base: UiMessageBase::new(UiMessageId::new("tc")),
        call_id: None,
        tool_name: String::new(),
        summary: None,
        arguments: None,
        state: UiToolCallState::default(),
    });
    assert_eq!(tool_call.kind(), UiMessageKind::ToolCall);

    let tool_result = UiMessage::ToolResult(UiToolResult {
        base: UiMessageBase::new(UiMessageId::new("tr")),
        call_id: None,
        tool_name: String::new(),
        content: String::new(),
        is_error: false,
    });
    assert_eq!(tool_result.kind(), UiMessageKind::ToolResult);

    let thinking = UiMessage::Thinking(UiThinkingBlock {
        base: UiMessageBase::new(UiMessageId::new("th")),
        summary: None,
        content: String::new(),
        timing: vec![],
        collapsed: false,
        phase: UiThinkingPhase::default(),
    });
    assert_eq!(thinking.kind(), UiMessageKind::Thinking);

    let step = UiMessage::Step(UiStep {
        base: UiMessageBase::new(UiMessageId::new("st")),
        step_index: 0,
        started_ms: 0,
        finished_ms: None,
        usage: UiTokenUsage::default(),
        finish_reason: None,
        model: None,
        state: UiStepState::default(),
    });
    assert_eq!(step.kind(), UiMessageKind::Step);

    let system = UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::new("sys")),
        text: String::new(),
        level: UiSystemMessageLevel::default(),
    });
    assert_eq!(system.kind(), UiMessageKind::System);

    let error = UiMessage::Error(UiErrorMessage {
        base: UiMessageBase::new(UiMessageId::new("err")),
        message: String::new(),
        recoverable: false,
    });
    assert_eq!(error.kind(), UiMessageKind::Error);
}

#[test]
fn ui_token_usage_default_is_all_zeros() {
    let usage = UiTokenUsage::default();
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.cached_tokens, 0);
    assert_eq!(usage.reasoning_tokens, 0);
}

#[test]
fn ui_tool_result_is_error_field_distinguishes_success_from_failure() {
    let ok_msg = UiMessage::ToolResult(UiToolResult {
        base: UiMessageBase::new(UiMessageId::new("ok")),
        call_id: None,
        tool_name: "read".to_string(),
        content: "data".to_string(),
        is_error: false,
    });
    assert_eq!(ok_msg.kind(), UiMessageKind::ToolResult);
    assert!(!match ok_msg {
        UiMessage::ToolResult(ref r) => r.is_error,
        _ => unreachable!(),
    });

    let err_msg = UiMessage::ToolResult(UiToolResult {
        base: UiMessageBase::new(UiMessageId::new("fail")),
        call_id: None,
        tool_name: "read".to_string(),
        content: "error".to_string(),
        is_error: true,
    });
    assert!(match err_msg {
        UiMessage::ToolResult(ref r) => r.is_error,
        _ => unreachable!(),
    });
}

#[test]
fn ui_message_id_from_str_and_string_produce_same_value() {
    let from_str = UiMessageId::new("my-id");
    assert_eq!(from_str.as_str(), "my-id");

    let from_string = UiMessageId::new("my-id".to_string());
    assert_eq!(from_string.as_str(), "my-id");

    assert_eq!(from_str, from_string);
}
