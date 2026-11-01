//! Permission verdicts cannot rewrite arguments or grant persistent capabilities.
use anyhow::{Result, ensure};
use serde_json::Value;

pub(crate) fn decision(output: &Value) -> Result<Option<bool>> {
    let Some(specific) = output.get("hookSpecificOutput") else { return Ok(None) };
    ensure!(specific["hookEventName"] == "PermissionRequest", "invalid permission hook event");
    let Some(decision) = specific.get("decision") else { return Ok(None) };
    ensure!(decision.is_object(), "invalid permission hook decision");
    ensure!(
        decision.get("updatedInput").is_none()
            && decision.get("updatedPermissions").is_none()
            && decision.get("interrupt").is_none_or(|v| v == false),
        "unsupported permission hook capability"
    );
    match decision["behavior"].as_str() {
        Some("allow") => Ok(Some(true)),
        Some("deny") => Ok(Some(false)),
        _ => anyhow::bail!("invalid permission hook behavior"),
    }
}
