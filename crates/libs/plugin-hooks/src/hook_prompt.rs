//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        input["hook_event_name"] == "UserPromptSubmit"
            && ["session_id", "turn_id", "model"]
                .iter()
                .all(|key| input[key].as_str().is_some_and(|s| !s.trim().is_empty()))
            && input["cwd"].as_str().is_some_and(|s| std::path::Path::new(s).is_absolute())
            && input["prompt"].is_string(),
        "invalid UserPromptSubmit input"
    );
    Ok(())
}

pub fn parse(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Result<Value> {
    if code == Some(2) {
        let reason = String::from_utf8_lossy(stderr);
        anyhow::bail!("UserPromptSubmit blocked: {}", reason.trim());
    }
    if code != Some(0) {
        return super::hook_command::interpret(code, &[]);
    }
    let text = String::from_utf8_lossy(stdout);
    let text = text.trim();
    if text.is_empty() {
        return Ok(Value::Null);
    }
    let output = match serde_json::from_str::<Value>(text) {
        Ok(output) => output,
        Err(_) if text.starts_with('{') => anyhow::bail!("invalid UserPromptSubmit JSON output"),
        Err(_) => return Ok(json!({"hookSpecificOutput":{"additionalContext":text}})),
    };
    if let Some(specific) = output.get("hookSpecificOutput") {
        ensure!(
            specific["hookEventName"] == "UserPromptSubmit",
            "invalid UserPromptSubmit output event"
        );
        ensure!(
            specific.get("additionalContext").is_none_or(Value::is_string),
            "invalid UserPromptSubmit context"
        );
    }
    if output["continue"] == false {
        anyhow::bail!(
            "UserPromptSubmit stopped: {}",
            output["stopReason"].as_str().unwrap_or("hook stopped execution")
        );
    }
    if output["decision"] == "block" {
        let reason = output["reason"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("UserPromptSubmit block requires a reason"))?;
        anyhow::bail!("UserPromptSubmit blocked: {reason}");
    }
    Ok(output)
}
