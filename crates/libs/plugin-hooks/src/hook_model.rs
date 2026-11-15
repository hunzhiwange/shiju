//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        ["session_id", "turn_id", "model"]
            .iter()
            .all(|k| input[k].as_str().is_some_and(|s| !s.trim().is_empty()))
            && input["cwd"].as_str().is_some_and(|s| std::path::Path::new(s).is_absolute()),
        "invalid model request metadata"
    );
    ensure!(
        input["hook_event_name"] == "PreModelRequest"
            || (input["hook_event_name"] == "PostModelRequest"
                && matches!(input["status"].as_str(), Some("completed" | "failed" | "cancelled"))
                && input.get("token_usage").is_some()),
        "invalid model request event"
    );
    Ok(())
}

pub fn token_usage(input: i64, output: i64, cached: i64, reasoning: i64) -> Value {
    json!({"input_tokens": input, "output_tokens":output, "cached_input_tokens":cached,
        "cache_write_input_tokens":0, "reasoning_output_tokens":reasoning,
        "total_tokens":input.saturating_add(output)})
}
