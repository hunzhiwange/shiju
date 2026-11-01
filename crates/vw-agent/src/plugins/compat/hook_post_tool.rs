//! Validate PostToolUse output before retaining model context.
use anyhow::{Result, ensure};
use serde_json::Value;

pub(crate) fn validate(output: &Value) -> Result<()> {
    if let Some(specific) = output.get("hookSpecificOutput") {
        ensure!(specific["hookEventName"] == "PostToolUse", "invalid PostToolUse event");
        ensure!(
            specific.get("updatedMCPToolOutput").is_none(),
            "unsupported PostToolUse updatedMCPToolOutput"
        );
        if let Some(context) = specific.get("additionalContext") {
            ensure!(context.is_string(), "invalid PostToolUse additionalContext");
        }
    }
    Ok(())
}

/// Post-tool control is feedback about an already completed operation, never a retry.
pub(crate) fn parse(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Result<Value> {
    if code == Some(2) {
        return Ok(
            serde_json::json!({"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":format!("PostToolUse blocked: {}", String::from_utf8_lossy(stderr))}}),
        );
    }
    if code != Some(0) {
        return super::hook_command::interpret(code, stdout);
    }
    let Ok(mut output) = serde_json::from_slice::<Value>(stdout) else { return Ok(Value::Null) };
    validate(&output)?;
    let blocked = output["decision"] == "block"
        || output["hookSpecificOutput"]["permissionDecision"] == "deny";
    let stopped = output["continue"] == false;
    if blocked || stopped {
        let reason = output["reason"]
            .as_str()
            .or_else(|| output["hookSpecificOutput"]["permissionDecisionReason"].as_str())
            .or_else(|| output["stopReason"].as_str())
            .unwrap_or("PostToolUse hook stopped execution")
            .to_owned();
        output = serde_json::json!({"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":reason}});
    }
    Ok(output)
}
