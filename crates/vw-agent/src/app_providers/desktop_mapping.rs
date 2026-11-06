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
        let mut row: Value = serde_json::from_str(&raw).map_err(|_| "invalid provider record")?;
        // Revalidate source credentials before changing how this row is interpreted.
        if desktop_mapping::import_row(&row)?.is_none() {
            return Err("unsupported provider".into());
        }
        if mapping.mode == DesktopMappingMode::Direct {
            let format =
                row.pointer("/meta/apiFormat").and_then(Value::as_str).unwrap_or("anthropic");
            if !matches!(format, "" | "anthropic")
                || row
                    .pointer("/settingsConfig/env/ANTHROPIC_AUTH_TOKEN")
                    .and_then(Value::as_str)
                    .is_none()
            {
                return Err("direct provider requires native bearer credentials".into());
            }
        }
        let meta = row
            .as_object_mut()
            .ok_or("invalid provider record")?
            .entry("meta")
            .or_insert(serde_json::json!({}))
            .as_object_mut()
            .ok_or("invalid provider metadata")?;
        meta.insert("claudeDesktopMode".into(), serde_json::to_value(mapping.mode).unwrap());
        meta.insert(
            "claudeDesktopModelRoutes".into(),
            serde_json::to_value(&mapping.routes).unwrap(),
        );
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
