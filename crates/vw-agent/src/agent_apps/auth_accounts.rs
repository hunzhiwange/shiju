//! Official account owner: metadata in shared tenant storage, secrets in private host files.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use std::path::{Path, PathBuf};
use vw_agent_apps::{
    apps::launch::{
        account::{AccountLaunch, prepare_account_launch},
        request::validate_launch_request,
    },
    auth::{
        credential_file::{AccountCredential, CredentialFileStore},
        credential_lease::CredentialLease,
    },
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
/// Holds the auth-owner lease through child exit, rotation writeback and temporary cleanup.
pub struct AuthAccountLaunch {
    launch: AccountLaunch,
    lease: CredentialLease,
}
impl AuthAccountLaunch {
    pub fn run(self) -> Result<(), String> {
        let Self { launch, lease } = self;
        let result = launch.run_with_refresh(|credential| lease.save(&credential));
        drop(lease);
        result
    }
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
        self.credentials.validate_material_scope(tenant, host, app, account_id, credential)?;
        let lease = self.credentials.lease(tenant, host, app, account_id, true)?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_accounts", &key, "{\"configured\":true}")
            .map_err(|_| "official account metadata unavailable")?;
        lease.save(credential)
    }
    pub fn register_claude_directory(
        &self,
        tenant: &TenantScope,
        host: &str,
        account_id: &str,
    ) -> Result<(), String> {
        let key = Self::key(host, AppId::Claude, account_id)?;
        self.credentials.register_claude_directory(tenant, host, account_id)?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_accounts", &key, "{\"configured\":true}")
            .map_err(|_| "official account metadata unavailable")?;
        Ok(())
    }
    /// An explicit scoped selection; never interprets the machine's current login as a default.
    pub fn set_default(
        &self,
        tenant: &TenantScope,
        host: &str,
        app: AppId,
        account_id: &str,
    ) -> Result<(), String> {
        let key = Self::key(host, app, account_id)?;
        if self
            .metadata
            .get(tenant, "agent_apps.auth_accounts", &key)
            .map_err(|_| "official account metadata unavailable")?
            .is_none()
        {
            return Err("official account not found".into());
        }
        self.credentials.read(tenant, host, app, account_id)?;
        let default_key =
            serde_json::to_string(&(host, app)).map_err(|_| "invalid official account identity")?;
        let value =
            serde_json::to_string(account_id).map_err(|_| "invalid official account identity")?;
        self.metadata
            .upsert(tenant, "agent_apps.auth_defaults", &default_key, &value)
            .map_err(|_| "official account metadata unavailable")?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_default_launch(
        &self,
        tenant: &TenantScope,
        host: &str,
        request: &LaunchRequest,
        installations: &[AppInstallation],
        roots: &[PathBuf],
        private_temp_root: &Path,
    ) -> Result<AuthAccountLaunch, String> {
        vw_agent_apps::providers::validate_identity(host)
            .map_err(|_| "invalid official account identity")?;
        let key = serde_json::to_string(&(host, request.app_id))
            .map_err(|_| "invalid official account identity")?;
        let value = self
            .metadata
            .get(tenant, "agent_apps.auth_defaults", &key)
            .map_err(|_| "official account metadata unavailable")?
            .ok_or("default official account not configured")?;
        let account_id: String =
            serde_json::from_str(&value).map_err(|_| "invalid stored default official account")?;
        self.prepare_launch(
            tenant,
            host,
            &account_id,
            request,
            installations,
            roots,
            private_temp_root,
        )
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
    ) -> Result<AuthAccountLaunch, String> {
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
        let lease = self.credentials.lease(tenant, host, request.app_id, account_id, false)?;
        let credential = lease.read()?;
        let launch =
            prepare_account_launch(request.app_id, credential, material, private_temp_root)?;
        Ok(AuthAccountLaunch { launch, lease })
    }
}
#[cfg(test)]
#[path = "auth_accounts_tests.rs"]
mod tests;
