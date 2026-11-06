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
    database: GlobalDatabase,
}
impl AppProviderStore {
    pub fn new(database: GlobalDatabase) -> Result<Self, String> {
        GatewaySettingsStore::new(database.clone())
            .map(|settings| Self { settings, database })
            .map_err(|_| "application provider storage unavailable".into())
    }
    fn key(app: AppId, id: &str) -> Result<String, String> {
        validate_identity(id)?;
        Ok(format!("{}:{id}", serde_json::to_string(&app).expect("AppId serialization")))
    }
    /// Consume the shared tenant library without linking provider records to deletable IDs.
    pub fn set_icon(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &vw_api_types::app_providers::icons::AppProviderIconChange,
    ) -> Result<vw_api_types::app_providers::icons::AppProviderIcon, String> {
        let key = Self::key(app, id)?;
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&before)
            .map_err(|_| "invalid stored application provider profile")?;
        let icons = match request {
            vw_api_types::app_providers::icons::AppProviderIconChange::Select { .. } => {
                crate::agent_apps::store::IconLibraryStore::new(self.database.clone())
                    .map_err(|_| "application provider storage unavailable")?
                    .read_icons(tenant)
                    .map_err(|_| "application provider storage unavailable")?
            }
            vw_api_types::app_providers::icons::AppProviderIconChange::Clear => Vec::new(),
        };
        let updated = vw_agent_apps::providers::icons::update(&stored, request, &icons)?;
        let after = serde_json::to_string(&updated)
            .map_err(|_| "invalid stored application provider profile")?;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &before, &after)
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider changed during update".into());
        }
        vw_agent_apps::providers::icons::read(&updated)
    }
    pub fn get_icon(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<vw_api_types::app_providers::icons::AppProviderIcon, String> {
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &Self::key(app, id)?)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&raw)
            .map_err(|_| "invalid stored application provider profile")?;
        vw_agent_apps::providers::icons::read(&stored)
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
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?;
        let input = vw_api_types::app_providers::update::AppProviderUpdate {
            name: Some(profile.name.clone()),
            protocol: Some(profile.protocol),
            api_key: vw_api_types::app_providers::create::ProviderSecretWrite::Replace {
                value: profile.api_key.clone(),
            },
            base_url: Some(profile.base_url.clone()),
            model: Some(profile.model.clone()),
            request_options: profile.request_options.clone(),
        };
        let stored = match &before {
            Some(raw) => serde_json::from_str(raw)
                .map_err(|_| "invalid stored application provider profile")?,
            None => {
                serde_json::to_value(profile).map_err(|_| "invalid application provider profile")?
            }
        };
        let updated = vw_agent_apps::providers::update::update(app, id, &stored, &input)?;
        let value =
            serde_json::to_string(&updated).map_err(|_| "invalid application provider profile")?;
        let saved = match before {
            Some(before) => {
                self.settings.compare_exchange(tenant, NAMESPACE, &key, &before, &value)
            }
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &value),
        }
        .map_err(|_| "application provider storage unavailable")?;
        if !saved {
            return Err("application provider changed during update".into());
        }
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
    /// Copy only within this tenant/application; atomic insertion never overwrites a target.
    pub fn duplicate(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &vw_api_types::app_providers::duplicate::AppProviderDuplicate,
    ) -> Result<AppProviderSummary, String> {
        let key = Self::key(app, id)?;
        let target = Self::key(app, &request.new_id)?;
        let source = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&source)
            .map_err(|_| "invalid stored application provider profile")?;
        vw_agent_apps::providers::duplicate::duplicate(app, id, &stored, request, |copy| {
            let payload =
                serde_json::to_string(copy).map_err(|_| "invalid application provider profile")?;
            self.settings
                .insert_if_absent(tenant, NAMESPACE, &target, &payload)
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
    /// Mutate only the existing scoped address book, never the active base URL.
    pub fn change_endpoint(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &vw_api_types::app_providers::endpoints::AppProviderEndpointChange,
    ) -> Result<Vec<vw_api_types::app_providers::endpoints::AppProviderEndpoint>, String> {
        let key = Self::key(app, id)?;
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&before)
            .map_err(|_| "invalid stored application provider profile")?;
        let updated = vw_agent_apps::providers::endpoints::update(
            &stored,
            request,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "application provider storage unavailable")?
                .as_millis() as u64,
        )?;
        let after = serde_json::to_string(&updated)
            .map_err(|_| "invalid stored application provider profile")?;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &before, &after)
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider changed during update".into());
        }
        vw_agent_apps::providers::endpoints::read(&updated)
    }
    pub fn list_endpoints(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<Vec<vw_api_types::app_providers::endpoints::AppProviderEndpoint>, String> {
        let value = self
            .settings
            .get(tenant, NAMESPACE, &Self::key(app, id)?)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&value)
            .map_err(|_| "invalid stored application provider profile")?;
        vw_agent_apps::providers::endpoints::read(&stored)
    }
    /// Replace presentation fields on the existing row using the same scoped conditional write.
    pub fn set_presentation(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        request: &vw_api_types::app_providers::presentation::AppProviderPresentation,
    ) -> Result<vw_api_types::app_providers::presentation::AppProviderPresentation, String> {
        let key = Self::key(app, id)?;
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&before)
            .map_err(|_| "invalid stored application provider profile")?;
        let updated = vw_agent_apps::providers::presentation::update(&stored, request)?;
        let after = serde_json::to_string(&updated)
            .map_err(|_| "invalid stored application provider profile")?;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &before, &after)
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider changed during update".into());
        }
        vw_agent_apps::providers::presentation::read(&updated)
    }
    /// Read category/grouping and notes without exposing the stored profile or credentials.
    pub fn list_presentation(
        &self,
        tenant: &TenantScope,
        app: AppId,
    ) -> Result<Vec<vw_api_types::app_providers::presentation::AppProviderPresentationEntry>, String>
    {
        use vw_api_types::app_providers::presentation::AppProviderPresentationEntry;
        let prefix = format!("{}:", serde_json::to_string(&app).expect("AppId serialization"));
        let mut rows = Vec::new();
        for entry in self
            .settings
            .list(tenant, NAMESPACE)
            .map_err(|_| "application provider storage unavailable")?
        {
            let Some(id) = entry.config_key.strip_prefix(&prefix) else {
                continue;
            };
            validate_identity(id)?;
            let stored = serde_json::from_str(&entry.payload_json)
                .map_err(|_| "invalid stored application provider profile")?;
            rows.push(AppProviderPresentationEntry {
                id: id.into(),
                presentation: vw_agent_apps::providers::presentation::read(&stored)?,
            });
        }
        rows.sort_by(|a, b| {
            (a.presentation.sort_index.is_none(), a.presentation.sort_index.unwrap_or_default())
                .cmp(&(
                    b.presentation.sort_index.is_none(),
                    b.presentation.sort_index.unwrap_or_default(),
                ))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(rows)
    }
    /// Delete only a scoped catalog row after checking the authorized host's native references.
    pub fn delete(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        home: &Path,
    ) -> Result<(), String> {
        let key = Self::key(app, id)?;
        self.settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        vw_agent_apps::providers::delete::ensure_unreferenced(home, app, id)?;
        if !self
            .settings
            .delete(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider not found".into());
        }
        Ok(())
    }
    /// Executor-only record for editor projection. Never returned by an HTTP handler.
    pub(crate) fn preview_record(
        database: &GlobalDatabase,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<serde_json::Value, String> {
        let raw =
            GatewaySettingsStore::read_existing(database, tenant, NAMESPACE, &Self::key(app, id)?)
                .map_err(|_| "application provider storage unavailable")?
                .ok_or("application provider not found")?;
        serde_json::from_str(&raw).map_err(|_| "invalid stored application provider profile".into())
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
        if app == AppId::Claude {
            if let Some(row) = self.claude_record(tenant, id)? {
                if let Some(config) = row.get("claude") {
                    let profile: vw_api_types::app_providers::claude::ClaudeProfile =
                        serde_json::from_value(config.clone())
                            .map_err(|_| "invalid stored application provider profile")?;
                    vw_agent_apps::providers::claude::settings(&profile)
                        .map_err(|_| "invalid stored application provider profile")?;
                    return Ok(AppProviderSummary {
                        app_id: app,
                        id: id.into(),
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::Codex {
            if let Some(row) = self.codex_record(tenant, id)? {
                if let Some(profile) = row.get("codex") {
                    let profile: vw_api_types::app_providers::codex_catalog::CodexProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::Gemini {
            if let Some(row) = self.gemini_record(tenant, id)? {
                if let Some(profile) = row.get("gemini") {
                    let profile: vw_api_types::app_providers::gemini::GeminiProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::OpenCode {
            if let Some(row) = self.opencode_record(tenant, id)? {
                if let Some(profile) = row.get("opencode") {
                    let profile: vw_api_types::app_providers::opencode::OpenCodeProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::OpenClaw {
            if let Some(row) = self.openclaw_record(tenant, id)? {
                if let Some(profile) = row.get("openclaw") {
                    let profile: vw_api_types::app_providers::openclaw::OpenClawProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::Hermes {
            if let Some(row) = self.hermes_record(tenant, id)? {
                if let Some(profile) = row.get("hermes") {
                    let profile: vw_api_types::app_providers::hermes::HermesProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::Pi {
            if let Some(row) = self.pi_record(tenant, id)? {
                if let Some(profile) = row.get("pi") {
                    let profile: vw_api_types::app_providers::pi::PiProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::Omp {
            if let Some(row) = self.omp_record(tenant, id)? {
                if let Some(profile) = row.get("omp") {
                    let profile: vw_api_types::app_providers::omp::OmpProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::GrokBuild {
            if let Some(row) = self.grok_record(tenant, id)? {
                if let Some(profile) = row.get("grok") {
                    let profile: vw_api_types::app_providers::grok::GrokProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
        if app == AppId::ClaudeDesktop {
            if let Some(row) = self.claude_desktop_record(tenant, id)? {
                if let Some(profile) = row.get("claude_desktop") {
                    let profile: vw_api_types::app_providers::claude_desktop::ClaudeDesktopProfile =
                        serde_json::from_value(profile.clone())
                            .map_err(|_| "invalid provider record")?;
                    return Ok(AppProviderSummary {
                        id: id.into(),
                        app_id: app,
                        name: profile.name,
                        configured: true,
                    });
                }
            }
        }
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

mod universal;

mod universal_sync;

impl AppProviderStore {
    pub(crate) fn editor_save_record(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        before: &serde_json::Value,
        settings: serde_json::Value,
    ) -> Result<(), String> {
        let key = Self::key(app, id)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|_| "invalid stored application provider profile")?;
        if stored != *before {
            return Err("application provider changed during update".into());
        }
        let mut after = stored;
        after["settingsConfig"] = settings;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &raw, &after.to_string())
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider changed during update".into());
        }
        Ok(())
    }
}

mod snippets;

mod claude;

mod codex_catalog;

mod gemini;

mod claude_desktop;
mod grok;

mod desktop_mapping;

mod hermes;
mod openclaw;
mod opencode;

mod pi;

mod omp;
