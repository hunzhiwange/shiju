//! PreToolUse updates remain subject to the host permission pipeline.
use anyhow::{Result, ensure};
use serde_json::Value;

pub(crate) fn updated_input(output: &Value) -> Result<Option<Value>> {
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
