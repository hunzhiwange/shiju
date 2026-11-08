//! Scoped authorization account selection and removal.

use super::{
    credential_file::{AccountCredential, CredentialFileStore},
    credential_lease::CredentialLease,
    flow::AuthAccountScope,
};

/// Remove a managed OAuth file while the caller holds its reference guard.
pub fn remove_managed_oauth_file(lease: &CredentialLease) -> Result<(), AuthErrorCode> {
    lease.remove_oauth_file().map_err(credential_error)
}

/// Delete the private Claude directory and its scoped keychain item under its lease.
#[cfg(target_os = "macos")]
pub fn remove_managed_claude(
    lease: &CredentialLease,
    identity_sha256: [u8; 32],
) -> Result<(), AuthErrorCode> {
    lease.remove_claude(&super::claude_keychain::MacOsKeychain, identity_sha256)
}
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
};

/// Tenant platform and company are supplied separately to the settings store.
pub fn managed_default_key(scope: AuthAccountScope<'_>) -> Result<String, AuthErrorCode> {
    scope.validate()?;
    match scope.service {
        AuthService::GithubCopilot | AuthService::CodexOauth | AuthService::XaiOauth => {
            serde_json::to_string(&(scope.principal, scope.host, scope.service))
                .map_err(|_| AuthErrorCode::InvalidInput)
        }
        _ => Err(AuthErrorCode::Unsupported),
    }
}

/// Hold the account's existing credential lease until the host has atomically
/// published the default. The publisher verifies the managed account metadata.
pub fn set_managed_default(
    credentials: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    account_id: &str,
    publish: impl FnOnce(&str) -> Result<(), AuthErrorCode>,
) -> Result<(), AuthErrorCode> {
    let key = managed_default_key(scope)?;
    crate::providers::validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    let lease = match scope.service {
        AuthService::GithubCopilot => credentials.lease_copilot(scope, account_id, false),
        AuthService::CodexOauth => credentials
            .for_owner(scope.principal)
            .map_err(credential_error)?
            .lease(scope.tenant, scope.host, AppId::Codex, account_id, false),
        AuthService::XaiOauth => credentials.lease_xai(scope, account_id, false),
        _ => return Err(AuthErrorCode::Unsupported),
    }
    .map_err(credential_error)?;
    if matches!(lease.read().map_err(credential_error)?, AccountCredential::XaiOauth(tokens) if tokens.requires_reauth)
    {
        return Err(AuthErrorCode::ReauthRequired);
    }
    publish(&key)
}

fn credential_error(error: String) -> AuthErrorCode {
    match error.as_str() {
        "official account not found" => AuthErrorCode::NotFound,
        "official account is in use" => AuthErrorCode::AccountInUse,
        _ => AuthErrorCode::StorageUnavailable,
    }
}
