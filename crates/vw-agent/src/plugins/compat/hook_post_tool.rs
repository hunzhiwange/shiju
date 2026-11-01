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
