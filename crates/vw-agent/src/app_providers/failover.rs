use super::{AppProviderStore, NAMESPACE};
use vw_agent_apps::{
    providers::failover::{ProviderSnapshot, snapshot},
    routing::supports_app,
};
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

impl AppProviderStore {
    /// Call after authorizing the project, execution host and app. Provider records
    /// are tenant-owned and shared across the tenant's authorized execution hosts.
    /// Each summary and credential classification come from the same saved row.
    pub fn failover_providers(
        &self,
        project: &ProjectScope,
        app: AppId,
    ) -> Result<Vec<ProviderSnapshot>, String> {
        if !supports_app(app) {
            return Err("application does not support routing".into());
        }
        let prefix = Self::prefix(app);
        let rows = self
            .settings
            .list(project.tenant_scope(), NAMESPACE)
            .map_err(|_| "application provider storage unavailable".to_string())?;
        rows.iter()
            .filter_map(|row| Some((row.config_key.strip_prefix(&prefix)?, &row.payload_json)))
            .map(|(id, raw)| {
                let row = serde_json::from_str(raw)
                    .map_err(|_| "invalid saved application provider profile")?;
                #[cfg(target_os = "macos")]
                let managed = app == AppId::Codex && super::has_codex_binding(&row);
                #[cfg(not(target_os = "macos"))]
                let managed = false;
                snapshot(app, id, &row, managed)
            })
            .collect()
    }
}
