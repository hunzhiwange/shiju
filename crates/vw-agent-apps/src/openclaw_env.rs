//! Sensitive OpenClaw env section. Executor/config authorization belongs to the
//! caller; raw reads additionally require its independent sensitive-read policy.
//! Candidate edits preserve non-target bytes using the shared JSON5 token parser.
use crate::{
    openclaw_model::OpenClawWriteOutcome,
    providers::{
        live::{
            commit::{AppState, ProviderTransactions},
            patch_json5::{object_value, replace_member_preserving_trivia},
            write_guard::WriteError,
        },
        openclaw::OpenClawFiles,
    },
};
use serde_json::{Map, Value};
use vw_api_types::id::TenantScope;

/// Complete object, including nested vars/shellEnv and unknown extensions.
/// No Debug implementation: incidental diagnostics must not expose secrets.
#[derive(Clone, PartialEq)]
pub struct OpenClawEnvConfig(Map<String, Value>);

impl OpenClawEnvConfig {
    /// Strict editor JSON; blank, malformed and non-object input are rejected.
    /// Parse errors intentionally discard source text and serde diagnostics.
    pub fn parse(raw: &str) -> Result<Self, WriteError> {
        let value: Value = serde_json::from_str(raw).map_err(|_| WriteError::InvalidResource)?;
        match value {
            Value::Object(object) => Ok(Self(object)),
            _ => Err(WriteError::InvalidResource),
        }
    }

    /// Sensitive payload: use only in an independently authorized response/editor.
    pub fn as_object(&self) -> &Map<String, Value> {
        &self.0
    }
}

fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let value = object_value(pre.unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    if value.get("env").is_some_and(|env| !env.is_object()) {
        return Err(WriteError::InvalidResource);
    }
    Ok(value)
}

/// The policy callback must check the authenticated scope's independent env
/// sensitive-read permission. It runs before parsing, even for a missing file.
/// Missing section is None; an explicitly cleared section is Some(empty).
pub fn get_env_config(
    pre: Option<&[u8]>,
    authorize_sensitive_read: impl FnOnce() -> Result<(), WriteError>,
) -> Result<Option<OpenClawEnvConfig>, WriteError> {
    authorize_sensitive_read()?;
    Ok(document(pre)?.get("env").and_then(Value::as_object).cloned().map(OpenClawEnvConfig))
}

/// Replace the complete env object; {} explicitly clears previous values.
/// This produces a sensitive candidate, never a list/log projection.
pub fn prepare_env_config(
    pre: Option<&[u8]>,
    env: &OpenClawEnvConfig,
) -> Result<Vec<u8>, WriteError> {
    document(pre)?;
    replace_member_preserving_trivia(pre.unwrap_or(b"{}"), "env", Value::Object(env.0.clone()))
        .map_err(|_| WriteError::InvalidResource)
}

/// Publish under the same physical lock and conflict-checked transaction as
/// provider/model/defaults writes. Caller must authorize config write and host.
pub fn set_env_config(
    files: &mut OpenClawFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    env: &OpenClawEnvConfig,
    unchanged: impl Fn() -> bool,
) -> Result<OpenClawWriteOutcome, WriteError> {
    let (name, pre) = files.snapshot()?;
    let next = prepare_env_config(pre.as_deref(), env)?;
    files.apply(tx, tenant, expected, name, pre.as_deref(), next, unchanged)?;
    Ok(OpenClawWriteOutcome { warnings: Vec::new() })
}
