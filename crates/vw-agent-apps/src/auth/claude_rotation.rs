//! Host-private Claude login status and generation checks.
use super::claude_keychain::ClaudeKeychain;
use crate::providers::{
    claude_login::LoginHost,
    live::{commit::ProviderTransactions, write_guard::WriteError},
};
use serde_json::Value;
use std::path::Path;
use vw_api_types::{
    TimestampMs, agent_apps::AppId, agent_apps_auth::AuthCredentialState, id::TenantScope,
};

#[derive(Debug, PartialEq, Eq)]
pub struct ClaudeLoginState {
    pub owner: Option<String>,
    pub credential_state: AuthCredentialState,
    pub login_expires_at: Option<TimestampMs>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClaudeLoginSwitchStatus {
    Active(ClaudeLoginState),
    OverrideDir,
}

/// Read the selected execution host's default slot under the same locks as a switch.
pub fn get_claude_login_switch(
    home: &Path,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    keychain: &dyn ClaudeKeychain,
    now: TimestampMs,
) -> Result<ClaudeLoginSwitchStatus, WriteError> {
    let _directory_lock = tx.lock_claude_directory()?;
    let state = tx.state(AppId::Claude)?;
    if !default_login_available(tx, tenant, &state)? {
        return Ok(ClaudeLoginSwitchStatus::OverrideDir);
    }
    keychain.authorize_home(home).map_err(|_| WriteError::InvalidResource)?;
    let host = LoginHost::open(home)?;
    if host.has_journal()? {
        return Err(WriteError::Conflict);
    }
    let material = host.material(keychain)?;
    let owner = material.identity()?;
    let login_expires_at = match &owner {
        Some(_) => expiry(material.oauth().and_then(|oauth| oauth.get("refreshTokenExpiresAt")))?,
        None => None,
    };
    let credential_state = match &owner {
        None => AuthCredentialState::SignedOut,
        Some(_) if login_expires_at.is_some_and(|at| at <= now) => {
            AuthCredentialState::ReauthRequired
        }
        Some(_) => AuthCredentialState::Ready,
    };
    Ok(ClaudeLoginSwitchStatus::Active(ClaudeLoginState {
        owner,
        credential_state,
        login_expires_at,
    }))
}

/// Check the default slot's scope while holding the Claude directory lock.
pub(crate) fn default_login_available(
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    state: &crate::providers::live::commit::AppState,
) -> Result<bool, WriteError> {
    if state.pointer.as_ref().is_some_and(|p| &p.tenant != tenant)
        || state.mode_tenant.as_ref().is_some_and(|owner| owner != tenant)
        || state.stack_tenant.as_ref().is_some_and(|owner| owner != tenant)
        || tx.pending(AppId::Claude)?.is_some()
    {
        return Err(WriteError::Conflict);
    }
    Ok(tx.claude_directory(tenant)?.is_none() && std::env::var_os("CLAUDE_CONFIG_DIR").is_none())
}

pub(crate) fn check_generation(
    before: Option<&Value>,
    incoming: Option<&Value>,
) -> Result<(), WriteError> {
    if before == incoming {
        return Ok(());
    }
    let incoming_expiry = expiry(incoming.and_then(|oauth| oauth.get("expiresAt")))?;
    let Some(before_expiry) = expiry(before.and_then(|oauth| oauth.get("expiresAt")))? else {
        // Older Claude snapshots may lack a generation timestamp. The locked
        // default slot remains authoritative for these same-owner snapshots.
        return Ok(());
    };
    let incoming_expiry = incoming_expiry.ok_or(WriteError::Conflict)?;
    let token_changed = ["accessToken", "refreshToken"].into_iter().any(|key| {
        before.and_then(|oauth| oauth.get(key)) != incoming.and_then(|oauth| oauth.get(key))
    });
    if incoming_expiry < before_expiry || (incoming_expiry == before_expiry && token_changed) {
        return Err(WriteError::Conflict);
    }
    Ok(())
}

pub(crate) fn expiry(value: Option<&Value>) -> Result<Option<TimestampMs>, WriteError> {
    let Some(value) = value else { return Ok(None) };
    let at = value
        .as_i64()
        .or_else(|| {
            value.as_f64().filter(|&at| at >= 1.0 && at < i64::MAX as f64).map(|at| at as i64)
        })
        .filter(|&at| at > 0)
        .ok_or(WriteError::InvalidResource)?;
    Ok(Some(TimestampMs(if at < 100_000_000_000 { at * 1000 } else { at })))
}
