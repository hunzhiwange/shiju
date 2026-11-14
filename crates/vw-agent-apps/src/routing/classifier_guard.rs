//! Guard against the frozen upstream auto-mode classifier retry loop.

use serde_json::Value;

pub const CLASSIFIER_LOOP_MESSAGE: &str = "ShiJu stopped this request after three consecutive tool-result turns reported an unavailable auto-mode safety classifier. No upstream request was sent. Fix the classifier model mapping or balance, then send a new user message to resume. Tool permission checks remain required.";

pub fn classifier_retry_loop(body: &Value) -> bool {
    if body
        .get("tools")
        .and_then(Value::as_array)
        .is_none_or(|tools| tools.is_empty())
    {
        return false;
    }
    let Some(messages) = body.get("messages").and_then(Value::as_array) else {
        return false;
    };
    let mut failed_turns = 0;
    let mut has_failed_results = false;
    for message in messages.iter().rev() {
        if message.get("role").and_then(Value::as_str) == Some("assistant") {
            if has_failed_results {
                failed_turns += 1;
                if failed_turns == 3 {
                    return true;
                }
                has_failed_results = false;
            }
            continue;
        }
        if message.get("role").and_then(Value::as_str) != Some("user") {
            return false;
        }
        let Some(blocks) = message.get("content").and_then(Value::as_array) else {
            return false;
        };
        if blocks.is_empty() || !blocks.iter().all(unavailable_classifier_result) {
            return false;
        }
        has_failed_results = true;
    }
    false
}

fn unavailable_classifier_result(block: &Value) -> bool {
    if block.get("type").and_then(Value::as_str) != Some("tool_result")
        || block.get("is_error").and_then(Value::as_bool) != Some(true)
    {
        return false;
    }
    fn matches_error(text: &str) -> bool {
        text.contains("is temporarily unavailable, so auto mode cannot determine the safety of ")
            || text.contains("The auto mode classifier is temporarily unavailable, so ")
    }
    match block.get("content") {
        Some(Value::String(text)) => matches_error(text),
        Some(Value::Array(blocks)) => blocks.iter().any(|block| {
            block.get("type").and_then(Value::as_str) == Some("text")
                && block
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(matches_error)
        }),
        _ => false,
    }
}
