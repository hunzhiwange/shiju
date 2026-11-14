//! agents.defaults editing under the existing OpenClaw configuration transaction.
//! Callers authorize the executor and config action before opening OpenClawFiles.
//! JSONC trivia is preserved. Unsupported JSON5 syntax fails closed in the shared
//! parser (single quotes and bare keys); full JSON5 support remains a shared-parser TODO.
use crate::providers::{
    live::{
        commit::{AppState, ProviderTransactions},
        patch_json::{object_value, replace_object_preserving_trivia},
        write_guard::WriteError,
    },
    openclaw::OpenClawFiles,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use vw_api_types::id::TenantScope;

/// Complete section, including unknown extensions and model/catalog fields.
/// Avoid Debug: extensions may contain sensitive application configuration.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpenClawAgentsDefaults(pub Map<String, Value>);

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenClawDefaultsForm {
    pub workspace: String,
    pub timeout_seconds: String,
    pub context_tokens: String,
    pub max_concurrent: String,
}

impl OpenClawAgentsDefaults {
    pub fn form(&self) -> OpenClawDefaultsForm {
        let number = |key: &str| self.0.get(key).filter(|v| v.is_number()).map(Value::to_string);
        OpenClawDefaultsForm {
            workspace: self.0.get("workspace").and_then(Value::as_str).unwrap_or("").into(),
            timeout_seconds: number("timeoutSeconds").or_else(|| number("timeout")).unwrap_or_default(),
            context_tokens: number("contextTokens").unwrap_or_default(),
            max_concurrent: number("maxConcurrent").unwrap_or_default(),
        }
    }

    /// Empty controls remove their fields. Invalid/non-finite numbers reject the
    /// entire edit; no partial section escapes. Other fields remain untouched.
    pub fn with_form(&self, form: &OpenClawDefaultsForm) -> Result<Self, WriteError> {
        let mut next = self.clone();
        let workspace = form.workspace.trim();
        if workspace.is_empty() {
            next.0.remove("workspace");
        } else {
            next.0.insert("workspace".into(), Value::String(workspace.into()));
        }
        for (key, input) in [
            ("timeoutSeconds", &form.timeout_seconds),
            ("contextTokens", &form.context_tokens),
            ("maxConcurrent", &form.max_concurrent),
        ] {
            if input.trim().is_empty() {
                next.0.remove(key);
            } else {
                let number = input.trim().parse::<f64>().map_err(|_| WriteError::InvalidResource)?;
                let number = serde_json::Number::from_f64(number).ok_or(WriteError::InvalidResource)?;
                next.0.insert(key.into(), Value::Number(number));
            }
        }
        next.0.remove("timeout");
        Ok(next)
    }
}

fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let value = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    for path in ["/agents", "/agents/defaults"] {
        if value.pointer(path).is_some_and(|v| !v.is_object()) {
            return Err(WriteError::InvalidResource);
        }
    }
    Ok(value)
}

pub fn get_agents_defaults(pre: Option<&[u8]>) -> Result<Option<OpenClawAgentsDefaults>, WriteError> {
    let value = document(pre)?;
    Ok(value.pointer("/agents/defaults").and_then(Value::as_object).cloned().map(OpenClawAgentsDefaults))
}

/// Full-section replacement; callers carry extensions from get_agents_defaults.
/// Only the known numeric legacy timeout is migrated, never unrelated old data.
pub fn prepare_agents_defaults(
    pre: Option<&[u8]>,
    defaults: &OpenClawAgentsDefaults,
) -> Result<Vec<u8>, WriteError> {
    let mut value = document(pre)?;
    let mut defaults = defaults.0.clone();
    if let Some(timeout) = defaults.get("timeout").filter(|v| v.is_number()).cloned() {
        if !defaults.get("timeoutSeconds").is_some_and(Value::is_number) {
            defaults.insert("timeoutSeconds".into(), timeout);
        }
        defaults.remove("timeout");
    }
    let agents = value.as_object_mut().ok_or(WriteError::InvalidResource)?
        .entry("agents").or_insert_with(|| serde_json::json!({}))
        .as_object_mut().ok_or(WriteError::InvalidResource)?;
    agents.insert("defaults".into(), Value::Object(defaults));
    replace_object_preserving_trivia(pre.unwrap_or(b"{}"), value)
        .map_err(|_| WriteError::InvalidResource)
}

/// Snapshot, preparation and publication share the provider/model physical lock.
pub fn set_agents_defaults(
    files: &mut OpenClawFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    defaults: &OpenClawAgentsDefaults,
    unchanged: impl Fn() -> bool,
) -> Result<(), WriteError> {
    let (name, pre) = files.snapshot()?;
    let next = prepare_agents_defaults(pre.as_deref(), defaults)?;
    files.apply(tx, tenant, expected, name, pre.as_deref(), next, unchanged)
}
