//! Plugin hook protocol implementation.
use anyhow::{Result, ensure};
use serde_json::Value;
use std::path::Path;

pub fn validate(input: &Value) -> Result<()> {
    ensure!(
        input["hook_event_name"] == "SessionStart"
            && input["session_id"].as_str().is_some_and(|s| !s.trim().is_empty())
            && input["cwd"].as_str().is_some_and(|s| Path::new(s).is_absolute())
            && matches!(input["source"].as_str(), Some("startup" | "resume")),
        "invalid session start input"
    );
    Ok(())
}
