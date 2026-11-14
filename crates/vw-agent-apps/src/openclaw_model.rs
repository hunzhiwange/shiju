//! Default-model section preparation. Publication uses the provider configuration
//! lock and transaction; callers authorize the executor and configuration action.
//! The existing round-trip parser accepts JSONC, and rejects unsupported JSON5
//! syntax rather than rewriting the document or silently losing comments.
// TODO(task 16): the shared provider parser must support full JSON5 before
// claiming JSON5 acceptance; single quotes/unquoted keys remain rejected.
use crate::providers::{
    live::{commit::{AppState, ProviderTransactions}, patch_json::{object_value, replace_object_preserving_trivia}, write_guard::WriteError},
    openclaw::OpenClawFiles,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use vw_api_types::id::TenantScope;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenClawDefaultModel {
    pub primary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenClawWriteOutcome {
    pub warnings: Vec<String>,
}

fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let value = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    for path in ["/agents", "/agents/defaults", "/agents/defaults/model"] {
        if value.pointer(path).is_some_and(|v| !v.is_object()) {
            return Err(WriteError::InvalidResource);
        }
    }
    Ok(value)
}

/// Missing section is distinct from malformed data. No filesystem side effects.
pub fn get_default_model(pre: Option<&[u8]>) -> Result<Option<OpenClawDefaultModel>, WriteError> {
    let value = document(pre)?;
    value.pointer("/agents/defaults/model").cloned()
        .map(|v| serde_json::from_value(v).map_err(|_| WriteError::InvalidResource))
        .transpose()
}

/// Prepare only this section, retaining unknown fields and non-target bytes.
/// Extra fields supplied by the caller override matching existing extensions;
/// omitted extensions survive. Fallback order and duplicates are intentional.
pub fn prepare_default_model(
    pre: Option<&[u8]>,
    model: &OpenClawDefaultModel,
) -> Result<Vec<u8>, WriteError> {
    if model.extra.contains_key("primary") || model.extra.contains_key("fallbacks") {
        return Err(WriteError::InvalidResource);
    }
    let mut value = document(pre)?;
    let root = value.as_object_mut().ok_or(WriteError::InvalidResource)?;
    let defaults = root.entry("agents").or_insert_with(|| serde_json::json!({}))
        .as_object_mut().ok_or(WriteError::InvalidResource)?
        .entry("defaults").or_insert_with(|| serde_json::json!({}))
        .as_object_mut().ok_or(WriteError::InvalidResource)?;
    let section = defaults.entry("model").or_insert_with(|| serde_json::json!({}))
        .as_object_mut().ok_or(WriteError::InvalidResource)?;
    section.insert("primary".into(), Value::String(model.primary.clone()));
    if model.fallbacks.is_empty() {
        section.remove("fallbacks");
    } else {
        section.insert("fallbacks".into(), serde_json::json!(model.fallbacks));
    }
    section.extend(model.extra.clone());
    replace_object_preserving_trivia(pre.unwrap_or(b"{}"), value)
        .map_err(|_| WriteError::InvalidResource)
}

/// The supplied files retain the same physical lock used by provider/defaults
/// writers. Snapshot and preparation happen under that lock. The transaction
/// checks external edits and propagates conflicts/recovery failures unchanged.
pub fn set_default_model(
    files: &mut OpenClawFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    model: &OpenClawDefaultModel,
    unchanged: impl Fn() -> bool,
) -> Result<OpenClawWriteOutcome, WriteError> {
    let (name, pre) = files.snapshot()?;
    let next = prepare_default_model(pre.as_deref(), model)?;
    files.apply(tx, tenant, expected, name, pre.as_deref(), next, unchanged)?;
    Ok(OpenClawWriteOutcome { warnings: Vec::new() })
}
