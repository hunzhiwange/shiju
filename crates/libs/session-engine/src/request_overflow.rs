//! Request-specific overflow classification preserves replay safety metadata.
use serde_json::Value;
use message_types::AssistantError;

fn llm_error_message(error: &str) -> String {
    serde_json::from_str::<Value>(error)
        .ok()
        .and_then(|value| value.get("message").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| error.to_string())
}

/// Context overflow needs a compaction retry in the caller, never the generic retry loop.
fn replay_is_unsafe(error: &str) -> bool {
    matches!(serde_json::from_str::<AssistantError>(error), Ok(AssistantError::APIError { metadata: Some(metadata), .. }) if metadata.get("replay_safe").is_some_and(|value| value == "false"))
}

pub fn is_prompt_too_long_error(error: &str) -> bool {
    if replay_is_unsafe(error) {
        return false;
    }
    let normalized = llm_error_message(error).to_ascii_lowercase();
    [
        "context length",
        "context window",
        "prompt too long",
        "too many tokens",
        "context overflow",
        "context_length_exceeded",
        "input too long",
        "input length exceeds",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

/// A range error alone is ambiguous (empty input also violates [1, N]).
/// Recover only when this nonempty request demonstrably exceeds its reported bound.
pub fn input_length_range_max(error: &str) -> Option<u64> {
    let normalized = llm_error_message(error).to_ascii_lowercase();
    let range = normalized.split_once("range of input length should be [")?.1.split_once(']')?.0;
    let (min, max) = range.split_once(',')?;
    let min = min.trim().parse::<u64>().ok()?;
    let max = max.trim().parse::<u64>().ok()?;
    (min == 1 && max > min).then_some(max)
}

pub fn is_request_too_long_error(error: &str, messages: &[Value]) -> bool {
    if replay_is_unsafe(error) {
        return false;
    }
    if is_prompt_too_long_error(error) {
        return true;
    }
    let Some(max) = input_length_range_max(error) else { return false };
    // Count content/arguments, not JSON envelope bytes: an empty field must never
    // be "repaired" by deleting unrelated history. Bytes are a conservative upper
    // bound when a provider does not report whether its limit uses chars or tokens.
    fn text_bytes(value: &Value) -> u64 {
        match value {
            Value::String(text) => text.len() as u64,
            Value::Array(items) => {
                items.iter().fold(0_u64, |n, item| n.saturating_add(text_bytes(item)))
            }
            Value::Object(fields) => {
                fields.values().fold(0_u64, |n, item| n.saturating_add(text_bytes(item)))
            }
            _ => 0,
        }
    }
    if messages.iter().any(|item| {
        text_bytes(&item["content"]) == 0 && item["tool_calls"].as_array().is_none_or(Vec::is_empty)
    }) {
        return false;
    }
    messages.iter().fold(0_u64, |n, item| {
        n.saturating_add(text_bytes(&item["content"]))
            .saturating_add(text_bytes(&item["tool_calls"]))
    }) > max
}
