//! Codex supplier rows in the existing provider namespace, never model catalog persistence.
use super::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::codex_catalog::CodexProfile;
impl AppProviderStore {
    pub(crate) fn codex_record(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<Option<Value>, String> {
        self.settings
            .get(tenant, NAMESPACE, &Self::key(AppId::Codex, id)?)
            .map_err(|_| "provider storage unavailable")?
            .map(|s| serde_json::from_str(&s).map_err(|_| "invalid provider record".into()))
            .transpose()
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn codex_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        profile: &CodexProfile,
        before: Option<&Value>,
        catalog_revision: Option<&str>,
    ) -> Result<Value, String> {
        let key = Self::key(AppId::Codex, id)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "provider storage unavailable")?;
        let stored: Option<Value> = raw
            .as_ref()
            .map(|s| serde_json::from_str(s).map_err(|_| "invalid provider record"))
            .transpose()?;
        if stored.as_ref() != before {
            return Err("provider changed".into());
        }
        let mut value = stored.unwrap_or(json!({}));
        let obj = value.as_object_mut().ok_or("invalid provider record")?;
        for key in ["apiKey", "baseUrl", "protocol", "model", "requestOptions"] {
            obj.remove(key);
        }
        obj.insert("name".into(), json!(profile.name));
        obj.insert("modelCatalogRevision".into(), json!(catalog_revision));
        obj.insert("codex".into(), serde_json::to_value(profile).map_err(|_| "invalid profile")?);
        obj.insert(
            "settingsConfig".into(),
            vw_agent_apps::providers::codex_catalog::settings(profile),
        );
        let after = value.to_string();
        let saved = match raw {
            Some(raw) => self.settings.compare_exchange(tenant, NAMESPACE, &key, &raw, &after),
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &after),
        }
        .map_err(|_| "provider storage unavailable")?;
        if !saved {
            return Err("provider changed".into());
        }
        Ok(value)
    }
}
