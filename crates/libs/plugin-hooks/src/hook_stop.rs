//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
pub use agent_hooks::TurnEndOutcome as Outcome;

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        ["session_id", "turn_id", "model", "permission_mode"]
            .iter()
            .all(|key| input[key].as_str().is_some_and(|s| !s.trim().is_empty()))
            && input["cwd"].as_str().is_some_and(|s| std::path::Path::new(s).is_absolute())
            && matches!(input["hook_event_name"].as_str(), Some("Stop" | "Interrupt")),
        "invalid turn end hook input"
    );
    if input["hook_event_name"] == "Stop" {
        ensure!(input["stop_hook_active"].is_boolean(), "missing stop hook recursion state");
    }
    Ok(())
}

pub fn parse(code: Option<i32>, stdout: &[u8], stderr: &[u8], stop: bool) -> Result<Value> {
    if stop {
        if code == Some(2) {
            let reason = String::from_utf8_lossy(stderr);
            if reason.trim().is_empty() {
                tracing::warn!("Stop hook exited with code 2 without a continuation prompt");
                return Ok(Value::Null);
            }
            return Ok(json!({"decision":"block", "reason":reason.trim()}));
        }
        if code != Some(0) {
            tracing::warn!("Stop hook command failed");
            return Ok(Value::Null);
        }
        if stdout.iter().all(u8::is_ascii_whitespace) {
            return Ok(Value::Null);
        }
        let value = serde_json::from_slice::<Value>(stdout).unwrap_or(Value::Null);
        let valid = value.as_object().is_some_and(|object| {
            object.iter().all(|(key, item)| match key.as_str() {
                "continue" | "suppressOutput" => item.is_boolean(),
                "stopReason" | "systemMessage" | "reason" => item.is_null() || item.is_string(),
                "decision" => item.is_null() || item == "block",
                _ => false,
            })
        });
        if !valid
            || (value["continue"] != false
                && value["decision"] == "block"
                && !value["reason"].as_str().is_some_and(|s| !s.trim().is_empty()))
        {
            tracing::warn!("invalid Stop hook output");
            return Ok(Value::Null);
        }
        return Ok(value);
    }
    if code != Some(0) {
        tracing::warn!("interrupt hook command failed");
        return Ok(Value::Null);
    }
    if stdout.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Null);
    }
    let value = serde_json::from_slice::<Value>(stdout).unwrap_or(Value::Null);
    if !value.as_object().is_some_and(|o| {
        o.iter().all(|(k, v)| match k.as_str() {
            "continue" | "suppressOutput" => v.is_boolean(),
            "stopReason" | "systemMessage" => v.is_null() || v.is_string(),
            _ => false,
        })
    }) {
        tracing::warn!("invalid interrupt hook output");
        return Ok(Value::Null);
    }
    Ok(value)
}

pub fn outcome(output: &Value, stop: bool) -> Outcome {
    if !stop {
        return Outcome::default();
    }
    Outcome {
        stop: output["continue"] == false,
        contexts: if output["continue"] != false && output["decision"] == "block" {
            output["reason"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(|s| vec![s.to_owned()])
                .unwrap_or_default()
        } else {
            Vec::new()
        },
    }
}
