use super::{AppProviderStore, NAMESPACE};
use crate::gateway::settings_store::{GatewaySettingPreimage, GatewaySettingsStore};
use vw_agent_apps::{
    providers::failover::{ProviderSnapshot, snapshot},
    routing::supports_app,
};
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

pub(crate) struct FailoverProviderSnapshot {
    pub(crate) provider: ProviderSnapshot,
    pub(crate) preimage: GatewaySettingPreimage,
}

impl AppProviderStore {
    /// Call after authorizing the project, execution host and app. Provider records
    /// are tenant-owned and shared across the tenant's authorized execution hosts.
    /// Each summary and credential classification come from the same saved row.
    pub fn failover_providers(
        &self,
        project: &ProjectScope,
        app: AppId,
    ) -> Result<Vec<ProviderSnapshot>, String> {
        self.failover_configuration(project, app)
            .map(|entries| entries.into_iter().map(|entry| entry.provider).collect())
    }

    pub(crate) fn failover_configuration(
        &self,
        project: &ProjectScope,
        app: AppId,
    ) -> Result<Vec<FailoverProviderSnapshot>, String> {
        if !supports_app(app) {
            return Err("application does not support routing".into());
        }
        let prefix = Self::prefix(app);
        let rows = GatewaySettingsStore::new(self.database.clone())
            .and_then(|settings| settings.list(project.tenant_scope(), NAMESPACE))
            .map_err(|_| "application provider storage unavailable".to_string())?;
        rows.into_iter()
            .filter_map(|saved| {
                let id = saved.config_key.strip_prefix(&prefix)?.to_owned();
                Some((id, saved))
            })
            .map(|(id, saved)| {
                let row: serde_json::Value = serde_json::from_str(&saved.payload_json)
                    .map_err(|_| "invalid saved application provider profile")?;
                if row.get("shiju_provider_publication").is_some() {
                    return Err("application provider publication is pending".into());
                }
                #[cfg(target_os = "macos")]
                let managed = app == AppId::Codex && super::has_codex_binding(&row);
                #[cfg(not(target_os = "macos"))]
                let managed = false;
                let provider = snapshot(app, &id, &row, managed)?;
                Ok(FailoverProviderSnapshot {
                    provider,
                    preimage: GatewaySettingPreimage {
                        namespace: NAMESPACE,
                        config_key: saved.config_key,
                        payload_json: saved.payload_json,
                    },
                })
            })
            .collect()
    }
}
