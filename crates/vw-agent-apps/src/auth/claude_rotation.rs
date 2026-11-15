//! Host-private Claude login status and generation checks.
use super::claude_keychain::ClaudeKeychain;
use crate::providers::{
    claude_login::LoginHost,
    live::{commit::ProviderTransactions, write_guard::WriteError},
};
use std::path::Path;
use {
    vw_api_transport::TimestampMs,
    vw_api_app_identity::AppId,
    vw_api_app_auth::AuthCredentialState,
    vw_api_transport::id::TenantScope,
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

pub(crate) use app_auth_claude::claude_rotation::{check_generation, expiry};
