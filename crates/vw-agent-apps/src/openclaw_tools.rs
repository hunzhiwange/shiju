//! OpenClaw tools settings only; these values never grant ShiJu tool permissions.
//! Callers authorize config access and the executor before opening OpenClawFiles.
use crate::providers::{
    live::{
        commit::{AppState, ProviderTransactions},
        patch_json::{object_value, replace_member_preserving_trivia},
        write_guard::WriteError,
    },
    openclaw::OpenClawFiles,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use vw_api_types::id::TenantScope;

/// Unknown profiles and extensions survive editing; no implicit full profile.
/// No Debug because extensions can contain sensitive application settings.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenClawToolsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenClawToolsProfile<'a> {
    Unset,
    Minimal,
    Coding,
    Messaging,
    Full,
    Unknown(&'a str),
}

impl OpenClawToolsConfig {
    pub fn profile_kind(&self) -> OpenClawToolsProfile<'_> {
        match self.profile.as_deref() {
            None => OpenClawToolsProfile::Unset,
            Some("minimal") => OpenClawToolsProfile::Minimal,
            Some("coding") => OpenClawToolsProfile::Coding,
            Some("messaging") => OpenClawToolsProfile::Messaging,
            Some("full") => OpenClawToolsProfile::Full,
            Some(value) => OpenClawToolsProfile::Unknown(value),
        }
    }
}

/// Missing file/section differs from an explicitly configured empty section.
pub fn get_tools_config(pre: Option<&[u8]>) -> Result<Option<OpenClawToolsConfig>, WriteError> {
    let value = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    value.get("tools").map(|tools| {
        serde_json::from_value(tools.clone()).map_err(|_| WriteError::InvalidResource)
    }).transpose()
}

/// Build a candidate without IO. Shared parser supports JSON/JSONC; unsupported
/// JSON5 syntax fails closed. Non-target text is retained by the shared patcher.
pub fn prepare_tools_config(
    pre: Option<&[u8]>,
    tools: &OpenClawToolsConfig,
) -> Result<Vec<u8>, WriteError> {
    get_tools_config(pre)?;
    if ["profile", "allow", "deny"].iter().any(|key| tools.extra.contains_key(*key)) {
        return Err(WriteError::InvalidResource);
    }
    let mut tools = tools.clone();
    // Match the source form: drop blank entries, preserve nonblank text and order.
    tools.allow.retain(|value| !value.trim().is_empty());
    tools.deny.retain(|value| !value.trim().is_empty());
    let value = serde_json::to_value(tools).map_err(|_| WriteError::InvalidResource)?;
    replace_member_preserving_trivia(pre.unwrap_or(b"{}"), "tools", value)
        .map_err(|_| WriteError::InvalidResource)
}

/// Snapshot and publication use the provider/model physical lock and conflict
/// checks. Full-section replacement requires carrying extensions from the read.
pub fn set_tools_config(
    files: &mut OpenClawFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    tools: &OpenClawToolsConfig,
    unchanged: impl Fn() -> bool,
) -> Result<(), WriteError> {
    let (name, pre) = files.snapshot()?;
    let next = prepare_tools_config(pre.as_deref(), tools)?;
    files.apply(tx, tenant, expected, name, pre.as_deref(), next, unchanged)
}
