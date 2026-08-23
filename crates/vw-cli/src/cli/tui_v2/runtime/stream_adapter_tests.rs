use serde_json::json;
use vw_gateway_client::{GatewayChatStreamEvent, GatewayChatUsage};

use super::stream_adapter::{
    UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_gateway_stream_event, contains_terminal_keyword,
};

fn usage() -> GatewayChatUsage {
    GatewayChatUsage { input_tokens: 1, output_tokens: 2, cached_tokens: 3, reasoning_tokens: 4 }
}

#[test]
fn done_terminal_preserves_metadata_and_normalizes_blank_finish_reason() {
    let event = adapt_gateway_stream_event(GatewayChatStreamEvent::Done {
        finish_reason: Some("  ".to_string()),
        usage: Some(json!({
            "input_tokens": 1,
            "output_tokens": 2,
            "cached_tokens": 3,
            "reasoning_tokens": 4
        })),
        message_id: Some("assistant_1".to_string()),
        parent_message_id: Some("user_1".to_string()),
    });

    assert_eq!(
        event,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done {
            finish_reason: None,
            usage: Some(usage()),
            message_id: Some("assistant_1".to_string()),
            parent_message_id: Some("user_1".to_string()),
        })
    );
}

#[test]
fn done_terminal_classifies_cancel_and_timeout_finish_reasons() {
    let cancelled = adapt_gateway_stream_event(GatewayChatStreamEvent::Done {
        finish_reason: Some(" user cancelled ".to_string()),
        usage: None,
        message_id: None,
        parent_message_id: None,
    });
    let timed_out = adapt_gateway_stream_event(GatewayChatStreamEvent::Done {
        finish_reason: Some("deadline exceeded".to_string()),
        usage: None,
        message_id: Some("m".to_string()),
        parent_message_id: Some("p".to_string()),
    });

    assert!(matches!(
        cancelled,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Cancelled { .. })
    ));
    assert!(matches!(timed_out, UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::TimedOut { .. })));
}

#[test]
fn error_terminal_normalizes_empty_and_classifies_control_messages() {
    assert_eq!(
        UiRuntimeTerminalEvent::from_error_message("  ".to_string()),
        UiRuntimeTerminalEvent::Error("gateway stream failed".to_string())
    );
    assert!(matches!(
        UiRuntimeTerminalEvent::from_error_message("interrupted by user".to_string()),
        UiRuntimeTerminalEvent::Cancelled { .. }
    ));
    assert!(matches!(
        UiRuntimeTerminalEvent::from_error_message("request timeout".to_string()),
        UiRuntimeTerminalEvent::TimedOut { .. }
    ));
}

#[test]
fn structured_gateway_events_map_to_runtime_events() {
    let step_start = adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.step_start",
        "step_index": 7,
        "created_ms": 111,
        "model": "model-a"
    })));
    let step_finish = adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.step_finish",
        "step_index": 7,
        "finished_ms": 222,
        "usage": {
            "input_tokens": 1,
            "output_tokens": 2,
            "cached_tokens": 3,
            "reasoning_tokens": 4
        },
        "finish_reason": "stop",
        "model": "model-a"
    })));

    assert_eq!(
        step_start,
        UiRuntimeEvent::StepStart {
            step_index: 7,
            created_ms: 111,
            model: Some("model-a".to_string()),
        }
    );
    assert_eq!(
        step_finish,
        UiRuntimeEvent::StepFinish {
            step_index: 7,
            finished_ms: 222,
            usage: usage(),
            finish_reason: Some("stop".to_string()),
            model: Some("model-a".to_string()),
        }
    );
}

#[test]
fn task_metadata_usage_and_unknown_events_are_explicit() {
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.context_loaded",
            "paths": ["/work/AGENTS.md"]
        }))),
        UiRuntimeEvent::ContextLoaded { paths: vec!["/work/AGENTS.md".to_string()] }
    );
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.todo_updated",
            "session_id": "s1"
        }))),
        UiRuntimeEvent::TaskStateChanged { session_id: Some("s1".to_string()) }
    );
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.session_updated",
            "session_id": "s1",
            "title": "  "
        }))),
        UiRuntimeEvent::SessionMetadataChanged { session_id: Some("s1".to_string()), title: None }
    );
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.usage_updated",
            "session_id": "s1",
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "cached_tokens": 3,
                "reasoning_tokens": 4
            }
        }))),
        UiRuntimeEvent::UsageUpdated { session_id: Some("s1".to_string()), usage: usage() }
    );
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "custom.event"
        }))),
        UiRuntimeEvent::Unknown { event_type: Some("custom.event".to_string()) }
    );
}

#[test]
fn delta_event_maps_directly_to_runtime_delta() {
    let event = adapt_gateway_stream_event(GatewayChatStreamEvent::Delta("hello".to_string()));
    assert_eq!(event, UiRuntimeEvent::Delta("hello".to_string()));
}

#[test]
fn typed_tool_event_reuses_the_existing_tool_delta_pipeline() {
    let event = adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.tool",
        "step_index": 2,
        "call_id": "call-2",
        "name": "shell",
        "kind": "complete",
        "status": "completed",
        "occurred_ms": 123,
        "payload": {"status": "completed", "output": "done"}
    })));

    assert_eq!(
        event,
        UiRuntimeEvent::Delta(
            "tool shell\n{\"status\":\"completed\",\"output\":\"done\"}\n".to_string()
        )
    );
}

#[test]
fn error_event_maps_to_terminal_error_via_classifier() {
    // Error with blank text gets the fallback message
    let blank = adapt_gateway_stream_event(GatewayChatStreamEvent::Error("  ".to_string()));
    assert_eq!(
        blank,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Error(
            "gateway stream failed".to_string()
        ))
    );

    // Error with cancellation keyword maps to Cancelled
    let cancelled =
        adapt_gateway_stream_event(GatewayChatStreamEvent::Error("request cancelled".to_string()));
    assert!(matches!(
        cancelled,
        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Cancelled { .. })
    ));

    // Error with timeout keyword maps to TimedOut
    let timed_out = adapt_gateway_stream_event(GatewayChatStreamEvent::Error(
        "operation timed out".to_string(),
    ));
    assert!(matches!(timed_out, UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::TimedOut { .. })));
}

#[test]
fn question_raised_and_resolved_map_to_task_state_changed() {
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.question_raised",
            "session_id": "s2"
        }))),
        UiRuntimeEvent::TaskStateChanged { session_id: Some("s2".to_string()) }
    );
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.question_resolved",
            "session_id": "s3"
        }))),
        UiRuntimeEvent::TaskStateChanged { session_id: Some("s3".to_string()) }
    );
}

#[test]
fn title_updated_event_maps_to_session_metadata_changed() {
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.title_updated",
            "session_id": "s4",
            "title": "new title"
        }))),
        UiRuntimeEvent::SessionMetadataChanged {
            session_id: Some("s4".to_string()),
            title: Some("new title".to_string()),
        }
    );
}

#[test]
fn title_updated_normalizes_blank_title() {
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.title_updated",
            "session_id": "s5",
            "title": "  "
        }))),
        UiRuntimeEvent::SessionMetadataChanged { session_id: Some("s5".to_string()), title: None }
    );
}

#[test]
fn post_tool_round_event_maps_to_runtime_lifecycle_event() {
    assert_eq!(
        adapt_gateway_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.post_tool_round",
            "step_index": 3
        }))),
        UiRuntimeEvent::PostToolRound
    );
}

#[test]
fn contains_terminal_keyword_matches_any_keyword() {
    assert!(contains_terminal_keyword("request timeout occurred", &["timeout", "cancelled"]));
    assert!(contains_terminal_keyword("user cancelled session", &["timeout", "cancelled"]));
    assert!(!contains_terminal_keyword("normal stop", &["timeout", "cancelled"]));
    assert!(!contains_terminal_keyword("", &["timeout", "cancelled"]));
}

#[test]
fn done_with_non_cancel_non_timeout_finish_reason_uses_done() {
    let done = adapt_gateway_stream_event(GatewayChatStreamEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: None,
        message_id: None,
        parent_message_id: None,
    });
    assert!(matches!(done, UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done { .. })));
}

#[test]
fn from_error_message_returns_error_for_unrecognized_message() {
    let result = UiRuntimeTerminalEvent::from_error_message("unknown failure".to_string());
    assert_eq!(result, UiRuntimeTerminalEvent::Error("unknown failure".to_string()));
}
