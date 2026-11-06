//! Official account owner: metadata in shared tenant storage, secrets in private host files.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use std::path::{Path, PathBuf};
use vw_agent_apps::{
    apps::launch::{
        account::{AccountLaunch, prepare_account_launch},
        request::validate_launch_request,
    },
    auth::credential_file::{AccountCredential, CredentialFileStore},
};
use vw_api_types::{
    agent_apps::{AppId, AppInstallation, LaunchRequest},
    id::TenantScope,
};

/// Internal authorized-host service. Not a client-selected directory or account discovery API.
pub struct AuthAccountStore {
    metadata: GatewaySettingsStore,
    credentials: CredentialFileStore,
}
impl AuthAccountStore {
    pub fn new(database: GlobalDatabase, credential_root: &Path) -> Result<Self, String> {
        Ok(Self {
            metadata: GatewaySettingsStore::new(database)
                .map_err(|_| "official account metadata unavailable")?,
            credentials: CredentialFileStore::new(credential_root)?,
        })
    }
    fn key(host: &str, app: AppId, account_id: &str) -> Result<String, String> {
        for value in [host, account_id] {
            vw_agent_apps::providers::validate_identity(value)
                .map_err(|_| "invalid official account identity")?;
        }
        serde_json::to_string(&(host, app, account_id))
            .map_err(|_| "invalid official account identity".into())
    }
    /// Caller must authorize this explicit import for the tenant/host; no ambient adoption.
    pub fn save(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        account_id: &str,
        credential: &AccountCredential,
    ) -> Result<(), String> {
        let key = Self::key(host, app, account_id)?;
        self.credentials.save(tenant, host, app, account_id, credential)?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_accounts", &key, "{\"configured\":true}")
            .map_err(|_| "official account metadata unavailable")?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_launch(
        &self,
        tenant: &TenantScope,
        host: &str,
        account_id: &str,
        request: &LaunchRequest,
        installations: &[AppInstallation],
        roots: &[PathBuf],
        private_temp_root: &Path,
    ) -> Result<AccountLaunch, String> {
        let key = Self::key(host, request.app_id, account_id)?;
        if self
            .metadata
            .get(tenant, "agent_apps.auth_accounts", &key)
            .map_err(|_| "official account metadata unavailable")?
            .as_deref()
            != Some("{\"configured\":true}")
        {
            return Err("official account not found".into());
        }
        let material = validate_launch_request(request, host, installations, roots, false)?;
        let credential = self.credentials.read(tenant, host, request.app_id, account_id)?;
        prepare_account_launch(request.app_id, credential, material, private_temp_root)
    }
}
#[cfg(test)]
#[path = "auth_accounts_tests.rs"]
mod tests;
