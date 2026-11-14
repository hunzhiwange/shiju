//! OpenClaw model allowlist projection and byte-preserving candidate edits.
//! Callers authorize the executor and publish candidates through OpenClawFiles
//! and the existing provider transaction, retaining the shared physical lock.
use crate::providers::live::{
    patch_json::{object_value, replace_member_preserving_trivia},
    write_guard::WriteError,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenClawModelCatalogEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub type OpenClawModelCatalog = BTreeMap<String, OpenClawModelCatalogEntry>;

fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let value = object_value(pre.unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    // Refuse malformed ancestors instead of silently replacing user settings.
    for path in ["/agents", "/agents/defaults", "/agents/defaults/models"] {
        if value.pointer(path).is_some_and(|v| !v.is_object()) {
            return Err(WriteError::InvalidResource);
        }
    }
    Ok(value)
}

/// Missing models are None; an explicit empty allowlist is Some(empty).
pub fn get_model_catalog(pre: Option<&[u8]>) -> Result<Option<OpenClawModelCatalog>, WriteError> {
    document(pre)?.pointer("/agents/defaults/models")
        .map(|v| serde_json::from_value(v.clone()).map_err(|_| WriteError::InvalidResource))
        .transpose()
}

/// Build a candidate only: no filesystem access, implicit HOME or publication.
/// The catalog replaces the selected section, including an explicit empty map.
pub fn set_model_catalog(
    pre: Option<&[u8]>,
    catalog: &OpenClawModelCatalog,
) -> Result<Vec<u8>, WriteError> {
    let mut value = document(pre)?;
    // A flattened field must not shadow the typed alias during serialization.
    if catalog.values().any(|entry| entry.extra.contains_key("alias")) {
        return Err(WriteError::InvalidResource);
    }
    let models = serde_json::to_value(catalog).map_err(|_| WriteError::InvalidResource)?;
    let agents = value.as_object_mut().ok_or(WriteError::InvalidResource)?
        .entry("agents").or_insert_with(|| Value::Object(Map::new()));
    let defaults = agents.as_object_mut().ok_or(WriteError::InvalidResource)?
        .entry("defaults").or_insert_with(|| Value::Object(Map::new()));
    defaults.as_object_mut().ok_or(WriteError::InvalidResource)?
        .insert("models".into(), models);
    replace_member_preserving_trivia(pre.unwrap_or(b"{}"), "agents", agents.clone())
        .map_err(|_| WriteError::InvalidResource)
}
