//! Read-only Claude control marker observations on an authorized execution host.
//!
//! This is config.json, independent of settings.json and installed plugin manifests.
use crate::providers::{
    live::{
        commit::ProviderTransactions,
        write_guard::{WriteError, lock_directory},
    },
    paths::open_beneath,
};
use serde::Serialize;
use std::{ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState, id::TenantScope};

/// The only public configuration projection. No arbitrary fields or values escape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeConfigProjection {
    pub primary_api_key_present: bool,
    pub managed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "config", rename_all = "snake_case")]
pub enum ClaudeConfigStatus {
    Missing,
    Valid(ClaudeConfigProjection),
    Invalid,
}

/// Fixed errors exclude paths, parser diagnostics and source document bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaudeStatusError {
    Target(WriteError),
    InvalidConfig,
}

impl From<WriteError> for ClaudeStatusError {
    fn from(error: WriteError) -> Self {
        Self::Target(error)
    }
}

/// Caller supplies HOME and tenant from authenticated executor dispatch, never request
/// paths. A remote host must dispatch there; it cannot fall back to this executor.
/// Blocking locks follow the existing Claude directory -> physical config order.
pub fn get_claude_plugin_status(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    target_host: &str,
) -> Result<ClaudeConfigStatus, ClaudeStatusError> {
    if !home.is_absolute() || !tx.is_for_host(target_host) {
        return Err(WriteError::InvalidResource.into());
    }
    let _directory_lock = tx.lock_claude_directory()?;
    let state = tx.state(AppId::Claude)?;
    if state.pointer.as_ref().is_some_and(|p| &p.tenant != tenant)
        || state.mode_tenant.as_ref().is_some_and(|owner| owner != tenant)
        || state.stack_tenant.as_ref().is_some_and(|owner| owner != tenant)
        || tx.pending(AppId::Claude)?.is_some()
    {
        return Err(WriteError::Conflict.into());
    }
    let directory = match tx.claude_directory(tenant)? {
        Some(selected) => selected.open(home)?,
        None => match open_beneath(home, &home.join(".claude"), true) {
            Ok(directory) => directory,
            Err(ConfigPathState::Absent) => return Ok(ClaudeConfigStatus::Missing),
            Err(_) => return Err(WriteError::InvalidResource.into()),
        },
    };
    let guard = lock_directory(AppId::Claude, directory)?;
    // Shared primitive bounds reads to 1 MiB and refuses links and special files.
    let Some(bytes) = guard.read(OsStr::new("config.json"))? else {
        return Ok(ClaudeConfigStatus::Missing);
    };
    let Ok(serde_json::Value::Object(document)) = serde_json::from_slice(&bytes) else {
        return Ok(ClaudeConfigStatus::Invalid);
    };
    Ok(ClaudeConfigStatus::Valid(ClaudeConfigProjection {
        primary_api_key_present: document.contains_key("primaryApiKey"),
        managed: document.get("primaryApiKey").and_then(serde_json::Value::as_str) == Some("any"),
    }))
}

/// Invalid JSON is an explicit error rather than an absent or disabled config.
pub fn read_claude_plugin_config(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    target_host: &str,
) -> Result<Option<ClaudeConfigProjection>, ClaudeStatusError> {
    match get_claude_plugin_status(home, tx, tenant, target_host)? {
        ClaudeConfigStatus::Missing => Ok(None),
        ClaudeConfigStatus::Valid(config) => Ok(Some(config)),
        ClaudeConfigStatus::Invalid => Err(ClaudeStatusError::InvalidConfig),
    }
}

pub fn is_claude_plugin_applied(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    target_host: &str,
) -> Result<bool, ClaudeStatusError> {
    Ok(read_claude_plugin_config(home, tx, tenant, target_host)?.is_some_and(|config| config.managed))
}
