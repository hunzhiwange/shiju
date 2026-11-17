//! 网关客户端测试模块，覆盖端点拼接、SSE 分帧和流式事件归一化行为。

use serde_json::json;

use crate::endpoint::GatewayEndpoint;
use crate::http::directory_query;
use crate::stream::{
    GatewayChatPostToolRoundEvent, GatewayChatStepFinishEvent, GatewayChatStepStartEvent,
    GatewayChatStreamEvent, GatewayChatUsage, GatewayTypedChatStreamEvent,
    normalize_chat_stream_event, take_next_sse_event,
};

#[test]
fn gateway_endpoint_uses_loopback_for_blank_host() {
    let endpoint = GatewayEndpoint::new("   ", 8080);

    assert_eq!(endpoint.normalized_host(), "127.0.0.1");
    assert_eq!(endpoint.base_url(), "http://127.0.0.1:8080");
    assert_eq!(endpoint.describe(), "127.0.0.1:8080");
}

#[test]
fn gateway_endpoint_keeps_non_empty_host() {
    let endpoint = GatewayEndpoint::new("gateway.internal", 9000);

    assert_eq!(endpoint.normalized_host(), "gateway.internal");
    assert_eq!(endpoint.base_url(), "http://gateway.internal:9000");
}

#[test]
fn directory_query_ignores_empty_values() {
    assert!(directory_query(None).is_empty());
    assert!(directory_query(Some("   ")).is_empty());
    assert_eq!(
        directory_query(Some("/tmp/workspace")),
        vec![("directory".to_string(), "/tmp/workspace".to_string())]
    );
}

#[test]
fn take_next_sse_event_consumes_one_frame_and_preserves_remainder() {
    let mut buffer =
        "data: {\"type\":\"chat.delta\"}\n\ndata: {\"type\":\"chat.done\"}\n\n".to_string();

    let first = take_next_sse_event(&mut buffer);

    assert_eq!(first.as_deref(), Some("data: {\"type\":\"chat.delta\"}"));
    assert_eq!(buffer, "data: {\"type\":\"chat.done\"}\n\n");
}

#[test]
fn take_next_sse_event_supports_crlf_separator() {
    let mut buffer = "data: {\"type\":\"chat.delta\"}\r\n\r\nrest".to_string();

    let frame = take_next_sse_event(&mut buffer);

    assert_eq!(frame.as_deref(), Some("data: {\"type\":\"chat.delta\"}"));
    assert_eq!(buffer, "rest");
}





#[test]
fn normalize_chat_stream_event_types_step_payloads() {
    let step_start = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.step_start",
        "step_index": 2,
        "created_ms": 123,
        "model": "claude-3-7-sonnet"
    })));

    assert_eq!(
        step_start,
        GatewayTypedChatStreamEvent::StepStart(GatewayChatStepStartEvent {
            step_index: 2,
            created_ms: 123,
            model: Some("claude-3-7-sonnet".to_string()),
        })
    );

    let step_finish = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.step_finish",
        "step_index": 2,
        "finished_ms": 456,
        "usage": {
            "input_tokens": 11,
            "output_tokens": 22,
            "cached_tokens": 33,
            "reasoning_tokens": 44
        },
        "finish_reason": "stop",
        "model": "claude-3-7-sonnet"
    })));

    assert_eq!(
        step_finish,
        GatewayTypedChatStreamEvent::StepFinish(GatewayChatStepFinishEvent {
            step_index: 2,
            finished_ms: 456,
            usage: GatewayChatUsage {
                input_tokens: 11,
                output_tokens: 22,
                cached_tokens: 33,
                reasoning_tokens: 44,
            },
            finish_reason: Some("stop".to_string()),
            model: Some("claude-3-7-sonnet".to_string()),
        })
    );

    let post_tool_round = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.post_tool_round",
        "step_index": 2
    })));

    assert_eq!(
        post_tool_round,
        GatewayTypedChatStreamEvent::PostToolRound(GatewayChatPostToolRoundEvent { step_index: 2 })
    );
}

#[test]
fn normalize_chat_stream_event_types_done_usage() {
    let event = normalize_chat_stream_event(GatewayChatStreamEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: Some(json!({
            "input_tokens": 1,
            "output_tokens": 2,
            "cached_tokens": 3,
            "reasoning_tokens": 4
        })),
        message_id: Some("msg_assistant".to_string()),
        parent_message_id: Some("msg_user".to_string()),
    });

    assert_eq!(
        event,
        GatewayTypedChatStreamEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: Some(GatewayChatUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 3,
                reasoning_tokens: 4,
            }),
            message_id: Some("msg_assistant".to_string()),
            parent_message_id: Some("msg_user".to_string()),
        }
    );
}

#[test]
fn normalize_chat_stream_event_marks_unknown_payloads() {
    let event = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.unhandled",
        "value": 1
    })));

    assert_eq!(
        event,
        GatewayTypedChatStreamEvent::Unknown { event_type: Some("chat.unhandled".to_string()) }
    );
}

#[test]
fn normalize_chat_stream_event_types_task_and_session_payloads() {
    let todo_updated = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.todo_updated",
        "session_id": "session_1",
        "todo": {
            "id": "todo_1",
            "content": "Refresh todo panel",
            "status": "pending",
            "priority": "medium"
        }
    })));

    assert_eq!(
        todo_updated,
        GatewayTypedChatStreamEvent::TodoUpdated { session_id: Some("session_1".to_string()) }
    );

    let question_raised = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.question_raised",
        "session_id": "session_1",
        "question": {
            "id": "question_1",
            "session_id": "session_1",
            "kind": "approval",
            "title": "Approve edit",
            "status": "pending",
            "created_at_ms": 123
        }
    })));

    assert_eq!(
        question_raised,
        GatewayTypedChatStreamEvent::QuestionRaised { session_id: Some("session_1".to_string()) }
    );

    let title_updated = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.title_updated",
        "session_id": "session_1",
        "title": "Fresh title"
    })));

    assert_eq!(
        title_updated,
        GatewayTypedChatStreamEvent::TitleUpdated {
            session_id: Some("session_1".to_string()),
            title: "Fresh title".to_string(),
        }
    );

    let session_updated = normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
        "type": "chat.session_updated",
        "session_id": "session_1",
        "session": {
            "id": "session_1",
            "title": "Synced title"
        }
    })));

    assert_eq!(
        session_updated,
        GatewayTypedChatStreamEvent::SessionUpdated {
            session_id: Some("session_1".to_string()),
            title: Some("Synced title".to_string()),
        }
    );
}
