//! Publish account and default deletion through the existing tenant store.

use super::{
    AuthAccountScope, AuthAccountStore, AuthErrorCode, AuthService, CredentialLease,
    MANAGED_ACCOUNT_NAMESPACE, defaults::MANAGED_DEFAULT_NAMESPACE,
};
use vw_agent_apps::auth::accounts::{
    managed_default_key, remove_managed_claude, remove_managed_oauth_file,
};

impl AuthAccountStore {
    pub(crate) fn lease_device_removal(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<CredentialLease, AuthErrorCode> {
        self.read_metadata(&scope, account_id)?;
        match scope.service {
            AuthService::GithubCopilot => self.credentials.lease_copilot(scope, account_id, false),
            AuthService::XaiOauth => self.credentials.lease_xai(scope, account_id, false),
            _ => return Err(AuthErrorCode::Unsupported),
        }
        .map_err(|error| match error.as_str() {
            "official account is in use" => AuthErrorCode::AccountInUse,
            _ => AuthErrorCode::StorageUnavailable,
        })
    }

    /// The caller holds the scoped lease and has already checked all provider references.
    pub(crate) fn remove_managed_file_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        lease: &CredentialLease,
    ) -> Result<(), AuthErrorCode> {
        let metadata = self.read_metadata(&scope, account_id)?;
        let key = Self::managed_key(&scope, account_id)?;
        let default_key = match managed_default_key(scope) {
            Ok(key) => Some(key),
            Err(AuthErrorCode::Unsupported) => None,
            Err(error) => return Err(error),
        };
        let id = serde_json::to_string(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        self.metadata
            .delete_with_reference(
                scope.tenant,
                (MANAGED_ACCOUNT_NAMESPACE, &key),
                default_key.as_deref().map(|key| (MANAGED_DEFAULT_NAMESPACE, key, id.as_str())),
                || match scope.service {
                    AuthService::ClaudeOfficial => {
                        remove_managed_claude(lease, metadata.external_identity_sha256)
                    }
                    _ => remove_managed_oauth_file(lease),
                },
            )
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
    }
}
