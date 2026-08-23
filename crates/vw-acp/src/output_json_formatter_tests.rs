//! 验证 JSON 输出器跟踪读工具调用并按策略脱敏其响应内容。
//!
//! 内存写入器和协议消息 fixture 用于只观察格式化结果及缓冲状态。

use super::*;
use crate::read_output_suppression::SUPPRESSED_READ_OUTPUT;
use crate::types::{
    OutputErrorCode, OutputErrorOrigin, OutputErrorParams, OutputFormatter, OutputFormatterContext,
};
use serde_json::{Value, json};

// 下列行为用例覆盖格式选择、内容抑制、截断和错误输出分支。
#[test]
fn json_rpc_id_key_keeps_number_and_string_namespaces_distinct() {
    assert_eq!(json_rpc_id_key(&json!("7")).as_deref(), Some("s:7"));
    assert_eq!(json_rpc_id_key(&json!(7)).as_deref(), Some("n:7"));
    assert_eq!(json_rpc_id_key(&Value::Null), None);
}

#[test]
fn sanitize_read_result_replaces_string_content_only() {
    let sanitized = sanitize_read_result(&json!({"content": "secret", "other": 1}));
    assert_eq!(sanitized["content"], SUPPRESSED_READ_OUTPUT);

    let unchanged = sanitize_read_result(&json!({"content": {"nested": true}}));
    assert_eq!(unchanged["content"], json!({"nested": true}));
}

#[test]
fn sanitize_read_result_leaves_non_content_shapes_unchanged() {
    assert_eq!(sanitize_read_result(&json!("plain")), json!("plain"));
    assert_eq!(sanitize_read_result(&json!({"path": "/tmp/a"})), json!({"path": "/tmp/a"}));
}

#[test]
fn sanitize_tool_content_replaces_arrays_only() {
    let sanitized = sanitize_tool_content(&json!([{"type": "text", "text": "secret"}]));
    assert_eq!(
        sanitized,
        json!([
            {
                "type": "content",
                "content": {
                    "type": "text",
                    "text": SUPPRESSED_READ_OUTPUT
                }
            }
        ])
    );

    assert_eq!(sanitize_tool_content(&json!("secret")), json!("secret"));
}

#[test]
fn sanitize_tool_message_ignores_non_update_shapes() {
    for message in [
        json!("plain"),
        json!({"method": "session/update"}),
        json!({"params": "invalid"}),
        json!({"params": {}}),
        json!({"params": {"update": "invalid"}}),
    ] {
        assert_eq!(sanitize_tool_message(&message), message);
    }
}

#[test]
fn sanitize_tool_message_preserves_null_and_missing_payloads() {
    let message = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": "session-1",
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "rawOutput": null,
                "content": null
            },
            "other": true
        }
    });

    let sanitized = sanitize_tool_message(&message);
    assert_eq!(sanitized["params"]["update"]["rawOutput"], Value::Null);
    assert_eq!(sanitized["params"]["update"]["content"], Value::Null);
    assert_eq!(sanitized["params"]["other"], true);

    let message_without_payloads = json!({
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-2"
            }
        }
    });
    assert_eq!(sanitize_tool_message(&message_without_payloads), message_without_payloads);
}

#[test]
fn formatter_suppresses_tracked_read_response() {
    let mut formatter = JsonOutputFormatter::new(
        Vec::new(),
        true,
        Some(OutputFormatterContext { session_id: "s1".to_string() }),
    );

    let request = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": 1, "method": "fs/read_text_file", "params": {}}),
    );
    let response = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": 1, "result": {"content": "secret"}}),
    );

    assert_eq!(request["method"], "fs/read_text_file");
    assert_eq!(response["result"]["content"], SUPPRESSED_READ_OUTPUT);
}

#[test]
fn formatter_keeps_read_response_when_suppression_is_disabled() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), false, None);

    let request = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": "1", "method": "fs/read_text_file"}),
    );
    let response = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": "1", "result": {"content": "secret"}}),
    );

    assert_eq!(request["method"], "fs/read_text_file");
    assert_eq!(response["result"]["content"], "secret");
    assert_eq!(
        formatter.request_method_by_id.get("s:1").map(String::as_str),
        Some("fs/read_text_file")
    );
}

#[test]
fn formatter_uses_default_session_id_without_context_or_with_blank_context() {
    let formatter = JsonOutputFormatter::new(Vec::<u8>::new(), true, None);
    assert_eq!(formatter.session_id, DEFAULT_JSON_SESSION_ID);

    let formatter = JsonOutputFormatter::new(
        Vec::<u8>::new(),
        true,
        Some(OutputFormatterContext { session_id: "   ".to_string() }),
    );
    assert_eq!(formatter.session_id, DEFAULT_JSON_SESSION_ID);
}

#[test]
fn formatter_leaves_untracked_or_non_read_responses_unchanged() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    let untracked = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": 1, "result": {"content": "secret"}}),
    );
    assert_eq!(untracked["result"]["content"], "secret");

    formatter
        .sanitize_message_value(json!({"jsonrpc": "2.0", "id": 2, "method": "fs/write_text_file"}));
    let non_read = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": 2, "result": {"content": "secret"}}),
    );
    assert_eq!(non_read["result"]["content"], "secret");
}

#[test]
fn formatter_keeps_string_and_number_request_ids_distinct() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": "7", "method": "fs/read_text_file"}),
    );

    let mismatched = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": 7, "result": {"content": "secret"}}),
    );
    assert_eq!(mismatched["result"]["content"], "secret");
    assert_eq!(
        formatter.request_method_by_id.get("s:7").map(String::as_str),
        Some("fs/read_text_file")
    );

    let tracked = formatter.sanitize_message_value(
        json!({"jsonrpc": "2.0", "id": "7", "result": {"content": "secret"}}),
    );
    assert_eq!(tracked["result"]["content"], SUPPRESSED_READ_OUTPUT);
    assert!(!formatter.request_method_by_id.contains_key("s:7"));
}

#[test]
fn formatter_ignores_malformed_messages_when_tracking() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    for message in [
        json!("plain"),
        json!({"jsonrpc": "2.0", "id": 1, "result": {"content": "secret"}}),
        json!({"jsonrpc": "2.0", "method": "fs/read_text_file"}),
        json!({"jsonrpc": "2.0", "id": true, "method": "fs/read_text_file"}),
    ] {
        assert_eq!(formatter.sanitize_message_value(message.clone()), message);
    }

    assert!(formatter.request_method_by_id.is_empty());
    assert!(formatter.tool_state_by_id.is_empty());
}

#[test]
fn formatter_sanitizes_read_tool_by_kind_and_allows_null_kind_reset() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    let first = formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "kind": "read",
                "content": "secret",
                "rawOutput": {"content": "secret"}
            }
        }
    }));
    assert_eq!(first["params"]["update"]["content"], "secret");
    assert_eq!(first["params"]["update"]["rawOutput"]["content"], SUPPRESSED_READ_OUTPUT);

    let second = formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "kind": null,
                "content": [{"type": "text", "text": "visible"}],
                "rawOutput": {"content": "visible"}
            }
        }
    }));
    assert_eq!(second["params"]["update"]["rawOutput"]["content"], "visible");
}

#[test]
fn formatter_sanitizes_read_tool_content_when_title_marks_tool_as_read_like() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "title": "Read File"
            }
        }
    }));

    let update = formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "content": [{"type": "text", "text": "secret"}],
                "rawOutput": {"content": "secret"}
            }
        }
    }));

    assert_eq!(
        update["params"]["update"]["content"],
        json!([
            {
                "type": "content",
                "content": {
                    "type": "text",
                    "text": SUPPRESSED_READ_OUTPUT
                }
            }
        ])
    );
    assert_eq!(update["params"]["update"]["rawOutput"]["content"], SUPPRESSED_READ_OUTPUT);
}

#[test]
fn formatter_preserves_previous_read_kind_for_non_string_kind_updates() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "kind": "read"
            }
        }
    }));

    let update = formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "kind": 7,
                "content": [{"type": "text", "text": "secret"}],
                "rawOutput": {"content": "secret"}
            }
        }
    }));

    assert_eq!(update["params"]["update"]["content"][0]["content"]["text"], SUPPRESSED_READ_OUTPUT);
    assert_eq!(update["params"]["update"]["rawOutput"]["content"], SUPPRESSED_READ_OUTPUT);
}

#[test]
fn formatter_tracks_read_tool_title_across_updates() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    let start = formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "title": "Read File"
            }
        }
    }));
    assert_eq!(start["params"]["update"]["title"], "Read File");

    let update = formatter.sanitize_message_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "update": {
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "content": [{"type": "text", "text": "secret"}],
                "rawOutput": {"content": "secret"}
            }
        }
    }));
    assert_eq!(update["params"]["update"]["rawOutput"]["content"], SUPPRESSED_READ_OUTPUT);
}

#[test]
fn formatter_ignores_non_tool_updates() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    for message in [
        json!({"jsonrpc": "2.0", "method": "session/update"}),
        json!({"jsonrpc": "2.0", "method": "session/update", "params": "invalid"}),
        json!({"jsonrpc": "2.0", "method": "session/update", "params": {}}),
        json!({"jsonrpc": "2.0", "method": "session/update", "params": {"update": "invalid"}}),
        json!({"jsonrpc": "2.0", "method": "session/update", "params": {"update": {}}}),
        json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {"update": {"sessionUpdate": "agent_message"}}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {"update": {"sessionUpdate": "tool_call"}}
        }),
        json!({"jsonrpc": "2.0", "method": "other", "params": {"update": {}}}),
    ] {
        assert_eq!(formatter.sanitize_message_value(message.clone()), message);
    }
}

#[test]
fn formatter_updates_context_and_falls_back_for_blank_internal_session() {
    let mut formatter = JsonOutputFormatter::new(
        Vec::new(),
        false,
        Some(OutputFormatterContext { session_id: "  session-1  ".to_string() }),
    );
    assert_eq!(formatter.session_id, "session-1");

    formatter.set_context(OutputFormatterContext { session_id: "session-2".to_string() });
    assert_eq!(formatter.session_id, "session-2");

    formatter.set_context(OutputFormatterContext { session_id: " ".to_string() });
    assert_eq!(formatter.session_id, "session-2");

    formatter.session_id = " ".to_string();
    formatter.set_context(OutputFormatterContext { session_id: " ".to_string() });
    assert_eq!(formatter.session_id, DEFAULT_JSON_SESSION_ID);
}

#[test]
fn formatter_writes_error_responses_with_current_session_id() {
    let mut formatter = JsonOutputFormatter::new(
        Vec::new(),
        true,
        Some(OutputFormatterContext { session_id: " session-9 ".to_string() }),
    );

    formatter.on_error(OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: Some("DETAIL".to_string()),
        origin: Some(OutputErrorOrigin::Queue),
        message: "boom".to_string(),
        retryable: Some(true),
        acp: None,
        timestamp: Some("2026-06-29T00:00:00Z".to_string()),
    });

    let output = String::from_utf8(formatter.into_inner()).expect("output should be utf8");
    let payload: Value = serde_json::from_str(output.trim()).expect("line should be valid json");
    assert_eq!(payload["error"]["message"], "boom");
    assert_eq!(payload["error"]["data"]["detailCode"], "DETAIL");
    assert_eq!(payload["error"]["data"]["origin"], "queue");
    assert_eq!(payload["error"]["data"]["retryable"], true);
    assert_eq!(payload["error"]["data"]["timestamp"], "2026-06-29T00:00:00Z");
    assert_eq!(payload["error"]["data"]["sessionId"], "session-9");
}

#[test]
fn formatter_writes_json_lines_and_flushes() {
    let mut formatter = JsonOutputFormatter::new(Vec::new(), true, None);

    formatter.on_acp_message(
        serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "fs/read_text_file"
        }))
        .expect("message should deserialize"),
    );
    formatter.flush();

    let output = String::from_utf8(formatter.into_inner()).expect("output should be utf8");
    assert_eq!(output.lines().count(), 1);
    let payload: Value = serde_json::from_str(output.trim()).expect("line should be valid json");
    assert_eq!(payload["method"], "fs/read_text_file");
}

#[test]
fn create_json_output_formatter_uses_constructor_options() {
    let formatter = create_json_output_formatter(
        Vec::<u8>::new(),
        true,
        Some(OutputFormatterContext { session_id: "  session-3  ".to_string() }),
    );

    assert!(formatter.suppress_reads);
    assert_eq!(formatter.session_id, "session-3");
    assert!(formatter.request_method_by_id.is_empty());
    assert!(formatter.tool_state_by_id.is_empty());
    assert!(formatter.into_inner().is_empty());
}
