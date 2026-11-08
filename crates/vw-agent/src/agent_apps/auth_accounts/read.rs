//! Public account projection for an already authorized owner and execution host.

use super::{
    AuthAccountScope, AuthAccountStore, AuthErrorCode, AuthService, MANAGED_ACCOUNT_NAMESPACE,
};
use sha2::{Digest, Sha256};
use std::path::Path;
use vw_agent_apps::auth::credential_file::AccountCredential;
use vw_api_types::agent_apps_auth::{AuthAccount, AuthCredentialState};

impl AuthAccountStore {
    pub(crate) fn scoped_account_rows(
        &self,
        scope: AuthAccountScope<'_>,
    ) -> Result<Vec<(String, String)>, AuthErrorCode> {
        scope.validate()?;
        let rows = self
            .metadata
            .list(scope.tenant, MANAGED_ACCOUNT_NAMESPACE)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let mut scoped = Vec::new();
        for row in rows {
            let (principal, host, service, id): (String, String, AuthService, String) =
                serde_json::from_str(&row.config_key)
                    .map_err(|_| AuthErrorCode::StorageUnavailable)?;
            if principal == scope.principal && host == scope.host && service == scope.service {
                scoped.push((id, row.payload_json));
            }
        }
        Ok(scoped)
    }

    pub(crate) fn read_managed_accounts(
        &self,
        scope: AuthAccountScope<'_>,
        home: &Path,
    ) -> Result<Vec<AuthAccount>, AuthErrorCode> {
        let default = match self.managed_default_account(scope) {
            Ok(default) => default,
            Err(AuthErrorCode::Unsupported) => None,
            Err(error) => return Err(error),
        };
        let fallback_label = match scope.service {
            AuthService::GithubCopilot => "GitHub Copilot",
            AuthService::CodexOauth => "ChatGPT",
            AuthService::XaiOauth => "xAI",
            AuthService::KimiOauth => "Kimi",
            AuthService::ClaudeOfficial => "Claude",
            _ => return Err(AuthErrorCode::Unsupported),
        };
        let mut accounts = Vec::new();
        for (id, raw) in self.scoped_account_rows(scope)? {
            let metadata: super::AuthAccountMetadata =
                serde_json::from_str(&raw).map_err(|_| AuthErrorCode::StorageUnavailable)?;
            let material = self.credentials.read_managed_account(scope, &id)?;
            let credential_state = match material {
                None => AuthCredentialState::SignedOut,
                Some(AccountCredential::CodexAuth(tokens))
                    if !vw_agent_apps::auth::codex_device::managed_identity(&tokens).is_ok_and(
                        |identity| identity_matches(identity, &metadata.external_identity_sha256),
                    ) =>
                {
                    AuthCredentialState::ReauthRequired
                }
                Some(AccountCredential::XaiOauth(tokens)) if tokens.requires_reauth => {
                    AuthCredentialState::ReauthRequired
                }
                Some(AccountCredential::KimiOauth(tokens))
                    if Some(tokens.region) != metadata.region =>
                {
                    AuthCredentialState::ReauthRequired
                }
                Some(AccountCredential::ClaudeDirectory(directory)) => {
                    claude_state(home, directory.path(), &metadata.external_identity_sha256)?
                }
                Some(_) => AuthCredentialState::Ready,
            };
            accounts.push(AuthAccount {
                is_default: default.as_deref() == Some(&id)
                    && credential_state != AuthCredentialState::SignedOut,
                account_id: id,
                service: scope.service,
                label: metadata
                    .label
                    .filter(|label| {
                        !label.trim().is_empty()
                            && label.len() <= 256
                            && !label.chars().any(char::is_control)
                    })
                    .unwrap_or_else(|| fallback_label.into()),
                credential_state,
                authenticated_at: Some(metadata.authenticated_at),
                login_expires_at: None,
                github_domain: (scope.service == AuthService::GithubCopilot)
                    .then_some(metadata.github_domain)
                    .flatten(),
                region: (scope.service == AuthService::KimiOauth)
                    .then_some(metadata.region)
                    .flatten(),
            });
        }
        Ok(accounts)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn read_kimi_default(
        &self,
        scope: AuthAccountScope<'_>,
        native: &vw_agent_apps::auth::kimi_account::KimiNativeTarget<'_>,
        account_ids: &[String],
    ) -> Result<Option<String>, AuthErrorCode> {
        match native.default_managed_account(&self.credentials, scope, account_ids) {
            Ok(id) => Ok(Some(id)),
            Err(AuthErrorCode::NotFound | AuthErrorCode::Forbidden) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[cfg(target_os = "macos")]
fn claude_state(
    home: &Path,
    path: &Path,
    identity: &[u8; 32],
) -> Result<AuthCredentialState, AuthErrorCode> {
    use vw_agent_apps::{
        auth::claude_keychain::{ClaudeKeychain, MacOsKeychain},
        providers::claude_login::LoginMaterial,
    };
    let keychain = MacOsKeychain;
    keychain.authorize_home(home).map_err(|_| AuthErrorCode::KeychainUnavailable)?;
    let material = LoginMaterial::from_account(path, &keychain)
        .map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let current = material.identity().map_err(|_| AuthErrorCode::StorageUnavailable)?;
    let Some(current) = current else {
        return Ok(AuthCredentialState::ReauthRequired);
    };
    Ok(if identity_matches(current, identity) {
        AuthCredentialState::Ready
    } else {
        AuthCredentialState::ReauthRequired
    })
}

#[cfg(not(target_os = "macos"))]
fn claude_state(
    _home: &Path,
    _path: &Path,
    _identity: &[u8; 32],
) -> Result<AuthCredentialState, AuthErrorCode> {
    Err(AuthErrorCode::Unsupported)
}

fn identity_matches(identity: String, expected: &[u8; 32]) -> bool {
    let actual: [u8; 32] = Sha256::digest(identity).into();
    &actual == expected
}
