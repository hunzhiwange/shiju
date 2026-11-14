//! Read-only configuration diagnostics. Callers authorize configuration access
//! and supply the trusted execution host HOME; no ambient HOME fallback.
use crate::providers::{live::{patch_json::object_value, write_guard::WriteError}, openclaw};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenClawHealthWarning {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

fn warning(code: &str, message: &str, path: &str) -> OpenClawHealthWarning {
    OpenClawHealthWarning {
        code: code.into(), message: message.into(), path: Some(path.into()),
    }
}

/// Missing files yield no warnings; read/permission failures remain errors.
/// Uses the existing descriptor-based provider reader without creating files.
pub fn scan_openclaw_config_health(home: &Path) -> Result<Vec<OpenClawHealthWarning>, WriteError> {
    let source = openclaw::read(home)?;
    Ok(scan_openclaw_health_from_bytes(source.as_deref()))
}

/// Shared parser currently supports JSON/JSONC, not full JSON5. Parser details
/// and input values are deliberately excluded from diagnostics.
pub fn scan_openclaw_health_from_bytes(source: Option<&[u8]>) -> Vec<OpenClawHealthWarning> {
    let Some(source) = source else { return Vec::new() };
    match object_value(source) {
        Ok(value) => scan_openclaw_health_from_value(&value),
        Err(_) => vec![warning("config_parse_failed", "OpenClaw configuration could not be parsed by the supported JSON/JSONC parser.", "openclaw.json")],
    }
}

/// Diagnostics preserve source branch semantics and never mutate configuration.
pub fn scan_openclaw_health_from_value(config: &Value) -> Vec<OpenClawHealthWarning> {
    let mut warnings = Vec::new();
    if let Some(profile) = config.pointer("/tools/profile").and_then(Value::as_str) {
        if !matches!(profile, "minimal" | "coding" | "messaging" | "full") {
            warnings.push(warning("invalid_tools_profile", "tools.profile uses an unsupported value.", "tools.profile"));
        }
    }
    if config.pointer("/agents/defaults/timeout").is_some() {
        warnings.push(warning("legacy_agents_timeout", "agents.defaults.timeout is deprecated; use agents.defaults.timeoutSeconds.", "agents.defaults.timeout"));
    }
    for (pointer, code, path, message) in [
        ("/env/vars", "stringified_env_vars", "env.vars", "env.vars should be an object."),
        ("/env/shellEnv", "stringified_env_shell_env", "env.shellEnv", "env.shellEnv should be an object."),
    ] {
        if config.pointer(pointer).is_some_and(|value| !value.is_object()) {
            warnings.push(warning(code, message, path));
        }
    }
    warnings
}
