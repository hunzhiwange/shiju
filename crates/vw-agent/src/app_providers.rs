//! Tenant application-provider storage owner, separate from global model authentication.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use std::path::{Path, PathBuf};
use vw_agent_apps::{
    apps::launch::{
        provider::{ProviderLaunch, prepare_provider_launch},
        request::validate_launch_request,
    },
    providers::{validate_identity, validate_profile},
};
use vw_api_types::{
    agent_apps::{AppId, AppInstallation},
    app_providers::{AppProviderSummary, AppProviderWrite, ProviderLaunchRequest},
    id::TenantScope,
};

const NAMESPACE: &str = "app_providers.records";

/// Fixed domain namespace; no global auth, host environment, upstream database or arbitrary key access.
pub struct AppProviderStore {
    settings: GatewaySettingsStore,
}
impl AppProviderStore {
    pub fn new(database: GlobalDatabase) -> Result<Self, String> {
        GatewaySettingsStore::new(database)
            .map(|settings| Self { settings })
            .map_err(|_| "application provider storage unavailable".into())
    }
    fn key(app: AppId, id: &str) -> Result<String, String> {
        validate_identity(id)?;
        Ok(format!("{}:{id}", serde_json::to_string(&app).expect("AppId serialization")))
    }
    /// Upsert a tenant-owned API provider without activating it or changing any CLI live config.
    pub fn save(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        profile: &AppProviderWrite,
    ) -> Result<AppProviderSummary, String> {
        let key = Self::key(app, id)?;
        validate_profile(app, profile)?;
        let value =
            serde_json::to_string(profile).map_err(|_| "invalid application provider profile")?;
        self.settings
            .upsert(tenant, NAMESPACE, &key, &value)
            .map_err(|_| "application provider storage unavailable")?;
        Ok(Self::summary(app, id, profile))
    }
    /// Insert a new record without activation; existing records are never replaced.
    pub fn create(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &vw_api_types::app_providers::create::AppProviderCreate,
    ) -> Result<AppProviderSummary, String> {
        vw_agent_apps::providers::create::create(app, id, request, |profile| {
            let key = Self::key(app, id)?;
            let value = serde_json::to_string(profile)
                .map_err(|_| "invalid application provider profile")?;
            self.settings
                .insert_if_absent(tenant, NAMESPACE, &key, &value)
                .map_err(|_| "application provider storage unavailable".into())
        })
    }
    /// Update an existing scoped catalog record, without touching native configuration.
    pub fn update(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &vw_api_types::app_providers::update::AppProviderUpdate,
    ) -> Result<AppProviderSummary, String> {
        let key = Self::key(app, id)?;
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&before)
            .map_err(|_| "invalid stored application provider profile")?;
        let updated = vw_agent_apps::providers::update::update(app, id, &stored, request)?;
        let profile = vw_agent_apps::providers::update::profile(&updated)?;
        let after =
            serde_json::to_string(&updated).map_err(|_| "invalid application provider profile")?;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &before, &after)
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider changed during update".into());
        }
        Ok(Self::summary(app, id, &profile))
    }
    fn read(&self, tenant: &TenantScope, app: AppId, id: &str) -> Result<AppProviderWrite, String> {
        let key = Self::key(app, id)?;
        let value = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&value)
            .map_err(|_| "invalid stored application provider profile")?;
        let profile = vw_agent_apps::providers::update::profile(&stored)?;
        validate_profile(app, &profile)?;
        Ok(profile)
    }
    fn summary(app: AppId, id: &str, profile: &AppProviderWrite) -> AppProviderSummary {
        AppProviderSummary {
            app_id: app,
            id: id.into(),
            name: profile.name.clone(),
            configured: true,
        }
    }
    pub fn get_summary(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<AppProviderSummary, String> {
        let profile = self.read(tenant, app, id)?;
        Ok(Self::summary(app, id, &profile))
    }
    /// Only summaries from this tenant's formal records; no ambient provider discovery.
    pub fn list(
        &self,
        tenant: &TenantScope,
        app: AppId,
    ) -> Result<Vec<AppProviderSummary>, String> {
        let prefix = format!("{}:", serde_json::to_string(&app).expect("AppId serialization"));
        self.settings
            .list(tenant, NAMESPACE)
            .map_err(|_| "application provider storage unavailable".to_string())?
            .into_iter()
            .filter_map(|entry| entry.config_key.strip_prefix(&prefix).map(str::to_owned))
            .map(|id| self.get_summary(tenant, app, &id))
            .collect()
    }
    /// Internal execution-host entry point. The host/observations/roots and skip decision must
    /// come from the authenticated host caller, never from client fields. The API does not
    /// expose this as a new execution endpoint; task36 owns the host dispatch.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_launch(
        &self,
        tenant: &TenantScope,
        request: &ProviderLaunchRequest,
        target_host_id: &str,
        installations: &[AppInstallation],
        authorized_roots: &[PathBuf],
        skip_permissions_authorized: bool,
        private_temp_root: &Path,
    ) -> Result<ProviderLaunch, String> {
        let profile = self.read(tenant, request.launch.app_id, &request.provider_id)?;
        let material = validate_launch_request(
            &request.launch,
            target_host_id,
            installations,
            authorized_roots,
            skip_permissions_authorized,
        )?;
        prepare_provider_launch(request.launch.app_id, &profile, material, private_temp_root)
    }
}

#[cfg(test)]
#[path = "app_providers_tests.rs"]
mod tests;
