//! PreToolUse updates remain subject to the host permission pipeline.
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn updated_input(output: &Value) -> Result<Option<Value>> {
    let specific = &output["hookSpecificOutput"];
    let Some(updated) = specific.get("updatedInput") else {
        return Ok(None);
    };
    ensure!(
        specific["hookEventName"] == "PreToolUse" && updated.is_object(),
        "invalid Codex updatedInput output"
    );
    if specific["permissionDecision"] != "allow" {
        return Ok(None);
    }
    Ok(Some(updated.clone()))
}

#[cfg(test)]
#[path = "hook_pre_tool_tests.rs"]
mod tests;
