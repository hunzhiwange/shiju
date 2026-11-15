//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        ["session_id", "turn_id", "model"]
            .iter()
            .all(|key| input[key].as_str().is_some_and(|value| !value.trim().is_empty()))
            && input["cwd"].as_str().is_some_and(|value| std::path::Path::new(value).is_absolute())
            && matches!(input["trigger"].as_str(), Some("auto" | "manual"))
            && matches!(input["hook_event_name"].as_str(), Some("PreCompact" | "PostCompact")),
        "invalid compact hook metadata"
    );
    Ok(())
}

pub fn parse(code: Option<i32>, stdout: &[u8]) -> Result<Value> {
    if code != Some(0) {
        tracing::warn!("compact hook command failed");
        return Ok(Value::Null);
    }
    let value = serde_json::from_slice::<Value>(stdout).unwrap_or(Value::Null);
    let valid = value.as_object().is_some_and(|object| {
        object.iter().all(|(key, value)| match key.as_str() {
            "continue" | "suppressOutput" => value.is_boolean(),
            "stopReason" | "systemMessage" => value.is_null() || value.is_string(),
            _ => false,
        })
    });
    if !valid {
        if !stdout.iter().all(u8::is_ascii_whitespace) {
            tracing::warn!("invalid compact hook output");
        }
        return Ok(Value::Null);
    }
    if value["continue"] == false {
        anyhow::bail!(
            "compact hook stopped: {}",
            value["stopReason"].as_str().unwrap_or("hook stopped execution")
        );
    }
    Ok(value)
}
