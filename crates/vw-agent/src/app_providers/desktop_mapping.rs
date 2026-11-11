//! Mapping writes use the same tenant namespace/CAS as direct profiles.
use super::*;
use serde_json::Value;
use vw_agent_apps::providers::desktop_mapping;
use vw_api_types::app_providers::desktop_mapping::*;
impl AppProviderStore {
    pub(crate) fn desktop_import_claude(&self, tenant: &TenantScope) -> Result<usize, String> {
        let prefix = format!("{}:", serde_json::to_string(&AppId::Claude).unwrap());
        let mut imported = 0;
        for entry in
            self.settings.list(tenant, NAMESPACE).map_err(|_| "provider storage unavailable")?
        {
            let Some(id) = entry.config_key.strip_prefix(&prefix) else { continue };
            let key = Self::key(AppId::ClaudeDesktop, id)?;
            if self
                .settings
                .get(tenant, NAMESPACE, &key)
                .map_err(|_| "provider storage unavailable")?
                .is_some()
            {
                continue;
            }
            let source: Value =
                serde_json::from_str(&entry.payload_json).map_err(|_| "invalid provider record")?;
            let Some(row) = desktop_mapping::import_row(&source)? else { continue };
            if self
                .settings
                .insert_if_absent(tenant, NAMESPACE, &key, &row.to_string())
                .map_err(|_| "provider storage unavailable")?
            {
                imported += 1;
            }
        }
        Ok(imported)
    }
    pub(crate) fn desktop_mapping_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        mapping: &DesktopMapping,
    ) -> Result<(), String> {
        let key = Self::key(AppId::ClaudeDesktop, id)?;
        desktop_mapping::validate(mapping)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "provider storage unavailable")?
            .ok_or("provider not found")?;
        let row: Value = serde_json::from_str(&raw).map_err(|_| "invalid provider record")?;
        let row = desktop_mapping::with_mapping(&row, mapping)?;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &raw, &row.to_string())
            .map_err(|_| "provider storage unavailable")?
        {
            return Err("provider changed".into());
        }
        Ok(())
    }
}
