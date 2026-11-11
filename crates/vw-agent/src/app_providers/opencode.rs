//! OpenCode supplier rows in the existing provider namespace with compare-and-exchange saves.
use super::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::opencode::OpenCodeProfile;
impl AppProviderStore {
    /// Read all OpenCode profile inputs without initializing storage or exposing them over HTTP.
    pub(crate) fn opencode_revision_records(
        database: &GlobalDatabase,
        tenant: &TenantScope,
    ) -> Result<Vec<(String, Value)>, String> {
        let prefix = Self::prefix(AppId::OpenCode);
        GatewaySettingsStore::list_existing(database, tenant, NAMESPACE)
            .map_err(|_| "provider storage unavailable")?
            .into_iter()
            .filter_map(|row| {
                row.config_key.strip_prefix(&prefix).map(|id| (id.to_owned(), row.payload_json))
            })
            .map(|(id, raw)| {
                serde_json::from_str(&raw)
                    .map(|row| (id, row))
                    .map_err(|_| "invalid provider record".into())
            })
            .collect()
    }

    pub(crate) fn opencode_record(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<Option<Value>, String> {
        self.settings
            .get(tenant, NAMESPACE, &Self::key(AppId::OpenCode, id)?)
            .map_err(|_| "provider storage unavailable")?
            .map(|s| serde_json::from_str(&s).map_err(|_| "invalid provider record".into()))
            .transpose()
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn opencode_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        profile: &OpenCodeProfile,
        before: Option<&Value>,
        metadata: Option<Value>,
    ) -> Result<Value, String> {
        let key = Self::key(AppId::OpenCode, id)?;
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
        let mut value = metadata.or(stored).unwrap_or(json!({}));
        let obj = value.as_object_mut().ok_or("invalid provider record")?;
        for key in ["apiKey", "baseUrl", "protocol", "model", "requestOptions"] {
            obj.remove(key);
        }
        obj.insert("name".into(), json!(profile.name));
        obj.insert(
            "opencode".into(),
            serde_json::to_value(profile).map_err(|_| "invalid profile")?,
        );
        obj.insert("settingsConfig".into(), profile.declaration.clone());
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
