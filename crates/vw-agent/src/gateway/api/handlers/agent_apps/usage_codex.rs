//! Host-private binding and credential version for managed Codex quota.

use std::path::Path;

use vw_agent_apps::{
    auth::{codex_device::managed_identity, credential_file::CredentialFileStore},
    usage::{binding::UsageAccount, cache::UsageCacheKey},
};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::UsageQueryKind,
};

use crate::{
    agent_apps::{
        auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
        usage_settings::UsageSettingsScope,
    },
    app_providers::AppProviderStore,
};
use sha2::{Digest, Sha256};

use super::usage_refresh::UsageRefresh;

pub(super) struct CodexScope<'a> {
    pub(super) selection: UsageSettingsScope<'a>,
    pub(super) credential_root: &'a Path,
    pub(super) account: &'a str,
}

impl CodexScope<'_> {
    fn account_scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: self.selection.project.tenant_scope(),
            principal: self.selection.principal,
            host: self.selection.target_host,
            service: AuthService::CodexOauth,
        }
    }

    pub(super) fn require_binding(
        &self,
        service: &UsageRefresh,
    ) -> Result<AuthAccountMetadata, AuthErrorCode> {
        let scope = self.account_scope();
        let providers = AppProviderStore::new(service.database.clone())
            .map_err(|_| AuthErrorCode::StorageUnavailable)?;
        if providers.codex_account_binding(scope, self.selection.provider)?.as_deref()
            != Some(self.account)
        {
            return Err(AuthErrorCode::NotFound);
        }
        AuthAccountStore::new(service.database.clone(), self.credential_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .read_metadata(&scope, self.account)
    }

    pub(super) fn current_key(
        &self,
        service: &UsageRefresh,
    ) -> Result<Option<UsageCacheKey>, AuthErrorCode> {
        let metadata = self.require_binding(service)?;
        let store = self.credentials()?;
        Ok(store
            .codex_usage_version(self.account_scope(), self.account)?
            .map(|version| self.key(service, &metadata, version)))
    }

    pub(super) fn prepare(
        &self,
        service: &UsageRefresh,
    ) -> Result<(UsageCacheKey, serde_json::Value), AuthErrorCode> {
        let metadata = self.require_binding(service)?;
        let (auth, version) = self
            .credentials()?
            .codex_usage_snapshot(self.account_scope(), self.account)?
            .ok_or(AuthErrorCode::ReauthRequired)?;
        let identity: [u8; 32] = Sha256::digest(managed_identity(&auth)?.as_bytes()).into();
        if identity != metadata.external_identity_sha256 {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        Ok((self.key(service, &metadata, version), auth))
    }

    fn credentials(&self) -> Result<CredentialFileStore, AuthErrorCode> {
        CredentialFileStore::new(self.credential_root)
            .map_err(|_| AuthErrorCode::StorageUnavailable)
    }

    fn key(
        &self,
        service: &UsageRefresh,
        metadata: &AuthAccountMetadata,
        credential: [u8; 32],
    ) -> UsageCacheKey {
        let scope = self.selection;
        UsageCacheKey {
            project: scope.project.clone(),
            principal: scope.principal.into(),
            host: scope.target_host.into(),
            app: scope.app,
            provider: scope.provider.into(),
            kind: UsageQueryKind::OfficialSubscription,
            account: Some((AuthService::CodexOauth, UsageAccount::Managed(self.account.into()))),
            credential_version: service.generation(credential),
            configuration_version: service.generation((
                self.credential_root,
                metadata.external_identity_sha256,
                metadata.authenticated_at.0,
            )),
        }
    }
}
