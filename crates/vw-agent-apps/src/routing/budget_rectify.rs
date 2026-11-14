//! Repair only an explicit Messages thinking-budget minimum rejection.
use serde_json::{Value, json};

/// Returns a changed body, or `None` when replay cannot fix this rejection.
pub fn repair(body: &Value, status: u16, error: &[u8]) -> Option<Value> {
    if status != 400 || !body.is_object() {
        return None;
    }
    let payload: Value = serde_json::from_slice(error).ok()?;
    let message = payload.pointer("/error/message")
        .or_else(|| payload.get("message"))
        .and_then(Value::as_str)?
        .to_ascii_lowercase();
    if !(message.contains("budget_tokens") || message.contains("budget tokens"))
        || !message.contains("thinking")
        || !(message.contains("greater than or equal to 1024")
            || message.contains(">= 1024")
            || (message.contains("1024") && message.contains("input should be")))
        || body.pointer("/thinking/type").and_then(Value::as_str) == Some("adaptive")
    {
        return None;
    }
    let mut fixed = body.clone();
    if !fixed.get("thinking").is_some_and(Value::is_object) {
        fixed["thinking"] = json!({});
    }
    fixed["thinking"]["type"] = json!("enabled");
    fixed["thinking"]["budget_tokens"] = json!(32000);
    if body.get("max_tokens").and_then(Value::as_u64).is_none_or(|n| n < 32001) {
        fixed["max_tokens"] = json!(64000);
    }
    (fixed != *body).then_some(fixed)
}
