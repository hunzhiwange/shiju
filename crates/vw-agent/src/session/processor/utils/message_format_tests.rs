use crate::app::agent::session::llm;

#[test]
fn now_ms_is_close_to_system_time() {
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_millis() as u64;
    let now = super::now_ms();
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_millis() as u64;

    assert!(now >= before);
    assert!(now <= after.saturating_add(1));
}

#[test]
fn tool_calls_to_assistant_message_preserves_call_shape_and_reasoning() {
    let calls = vec![
        llm::ToolCall {
            id: "call-1".to_string(),
            name: "file_read".to_string(),
            arguments: r#"{"path":"src/lib.rs"}"#.to_string(),
        },
        llm::ToolCall {
            id: "call-2".to_string(),
            name: "bash".to_string(),
            arguments: r#"{"command":"echo ok"}"#.to_string(),
        },
    ];

    let message = super::tool_calls_to_assistant_message(&calls, "thinking");

    assert_eq!(
        message,
        serde_json::json!({
            "role": "assistant",
            "tool_calls": [
                {
                    "id": "call-1",
                    "type": "function",
                    "function": {
                        "name": "file_read",
                        "arguments": r#"{"path":"src/lib.rs"}"#
                    }
                },
                {
                    "id": "call-2",
                    "type": "function",
                    "function": {
                        "name": "bash",
                        "arguments": r#"{"command":"echo ok"}"#
                    }
                }
            ],
            "reasoning_content": "thinking"
        })
    );
}

#[test]
fn tool_calls_to_assistant_message_handles_empty_calls() {
    let message = super::tool_calls_to_assistant_message(&[], "");

    assert_eq!(
        message,
        serde_json::json!({
            "role": "assistant",
            "tool_calls": [],
            "reasoning_content": ""
        })
    );
}

#[test]
fn tool_calls_to_assistant_message_keeps_arguments_as_opaque_text() {
    let calls = vec![llm::ToolCall {
        id: "调用-\"1\"".to_string(),
        name: "自定义工具".to_string(),
        arguments: "not json\n{\"value\":\"雪\"}".to_string(),
    }];

    let message = super::tool_calls_to_assistant_message(&calls, "第一行\n第二行");

    assert_eq!(
        message,
        serde_json::json!({
            "role": "assistant",
            "tool_calls": [{
                "id": "调用-\"1\"",
                "type": "function",
                "function": {
                    "name": "自定义工具",
                    "arguments": "not json\n{\"value\":\"雪\"}"
                }
            }],
            "reasoning_content": "第一行\n第二行"
        })
    );
}

#[test]
fn tool_result_to_message_uses_tool_role_and_call_id() {
    let message = super::tool_result_to_message("call-9", "done");

    assert_eq!(message.get("role").and_then(serde_json::Value::as_str), Some("tool"));
    assert_eq!(message.get("tool_call_id").and_then(serde_json::Value::as_str), Some("call-9"));
    assert_eq!(message.get("content").and_then(serde_json::Value::as_str), Some("done"));
}

#[test]
fn tool_result_to_message_preserves_empty_content() {
    let message = super::tool_result_to_message("call-empty", "");

    assert_eq!(
        message,
        serde_json::json!({
            "role": "tool",
            "tool_call_id": "call-empty",
            "content": ""
        })
    );
}

#[test]
fn tool_result_to_message_keeps_json_like_content_as_text() {
    let message = super::tool_result_to_message("call-json", "{\"ok\":true}\n完成");

    assert_eq!(
        message,
        serde_json::json!({
            "role": "tool",
            "tool_call_id": "call-json",
            "content": "{\"ok\":true}\n完成"
        })
    );
}

#[test]
fn assistant_message_with_reasoning_includes_content_and_reasoning() {
    let message = super::assistant_message_with_reasoning("answer", "chain");

    assert_eq!(message.get("role").and_then(serde_json::Value::as_str), Some("assistant"));
    assert_eq!(message.get("content").and_then(serde_json::Value::as_str), Some("answer"));
    assert_eq!(message.get("reasoning_content").and_then(serde_json::Value::as_str), Some("chain"));
}

#[test]
fn assistant_message_with_reasoning_preserves_empty_content_and_multiline_reasoning() {
    let message = super::assistant_message_with_reasoning("", "分析一\n分析二");

    assert_eq!(
        message,
        serde_json::json!({
            "role": "assistant",
            "content": "",
            "reasoning_content": "分析一\n分析二"
        })
    );
}
