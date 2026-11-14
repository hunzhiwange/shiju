//! Hermes memory budgets and switches. Callers authorize configuration access
//! and the executor before reading or acquiring the provider's shared write lock.
use crate::providers::{
    hermes::HermesFiles,
    live::{
        commit::{AppState, ProviderTransactions},
        patch_yaml::patch_hermes_memory_enabled,
        write_guard::WriteError,
    },
};
use serde_yaml::Value;
use vw_api_types::{
    hermes_memory::{HermesMemoryKind, HermesMemoryLimits},
    id::TenantScope,
};

/// Parse an executor-owned config snapshot; invalid individual fields default,
/// while malformed documents fail without exposing their potentially secret text.
pub fn read_memory_limits(pre: Option<&[u8]>) -> Result<HermesMemoryLimits, WriteError> {
    let bytes = pre.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return Err(WriteError::InvalidResource);
    }
    let document: Value = serde_yaml::from_slice(bytes).map_err(|_| WriteError::InvalidResource)?;
    if !document.is_mapping() && !document.is_null() {
        return Err(WriteError::InvalidResource);
    }
    let memory = document.get("memory");
    let field = |key: &str| memory.and_then(|m| m.get(key));
    Ok(HermesMemoryLimits {
        memory: field("memory_char_limit").and_then(Value::as_u64).unwrap_or(2200),
        user: field("user_char_limit").and_then(Value::as_u64).unwrap_or(1375),
        memory_enabled: field("memory_enabled").and_then(Value::as_bool).unwrap_or(true),
        user_enabled: field("user_profile_enabled").and_then(Value::as_bool).unwrap_or(true),
    })
}

/// Prepare a single switch edit without reserializing non-target YAML. Ambiguous
/// layouts (flow sections, references, tags) require manual editing, never loss.
pub fn prepare_memory_enabled(
    pre: Option<&[u8]>,
    kind: HermesMemoryKind,
    enabled: bool,
) -> Result<Vec<u8>, WriteError> {
    read_memory_limits(pre)?;
    patch_hermes_memory_enabled(pre, kind, enabled).map_err(|_| WriteError::InvalidResource)
}

/// Publish through the same physical lock and conflict transaction as providers.
/// Budgets are read-only here; memory contents are never truncated or rewritten.
pub fn set_memory_enabled(
    files: &mut HermesFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    kind: HermesMemoryKind,
    enabled: bool,
    unchanged: impl Fn() -> bool,
) -> Result<(), WriteError> {
    let (name, pre) = files.snapshot()?;
    let next = prepare_memory_enabled(pre.as_deref(), kind, enabled)?;
    files.apply(tx, tenant, expected, name, pre.as_deref(), next, unchanged)
}
