//! 消息格式化辅助逻辑，负责在内部工具调用、工具结果和模型消息之间做结构转换。

use crate::app::agent::session::llm;
use serde_json::Value;

/// 执行 now_ms 操作，并返回调用方需要的结果。
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 执行 tool_calls_to_assistant_message 操作，并返回调用方需要的结果。
pub(crate) fn tool_calls_to_assistant_message(
    tool_calls: &[llm::ToolCall],
    reasoning_content: &str,
) -> Value {
    let calls = tool_calls
        .iter()
        .map(|c| {
            let mut call = serde_json::json!({
                "id": c.id,
                "type": "function",
                "function": { "name": c.name, "arguments": c.arguments }
            });
            if let Some(namespace) = &c.namespace {
                call["namespace"] = serde_json::json!(namespace);
            }
            call
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "role": "assistant",
        "tool_calls": calls,
        "reasoning_content": reasoning_content
    })
}

/// 执行 tool_result_to_message 操作，并返回调用方需要的结果。
pub(crate) fn tool_result_to_message(tool_call_id: &str, content: &str) -> Value {
    serde_json::json!({ "role": "tool", "tool_call_id": tool_call_id, "content": content })
}

/// 执行 assistant_message_with_reasoning 操作，并返回调用方需要的结果。
pub(crate) fn assistant_message_with_reasoning(content: &str, reasoning_content: &str) -> Value {
    serde_json::json!({
        "role": "assistant",
        "content": content,
        "reasoning_content": reasoning_content
    })
}
#[cfg(test)]
#[path = "message_format_tests.rs"]
mod message_format_tests;
