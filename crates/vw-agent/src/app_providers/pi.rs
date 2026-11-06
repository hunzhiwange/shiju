//! Pi supplier rows in the existing provider namespace with compare-and-exchange saves.
use super::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::pi::PiProfile;
impl AppProviderStore {
    pub(crate) fn pi_record(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<Option<Value>, String> {
        self.settings
            .get(tenant, NAMESPACE, &Self::key(AppId::Pi, id)?)
            .map_err(|_| "provider storage unavailable")?
            .map(|s| serde_json::from_str(&s).map_err(|_| "invalid provider record".into()))
            .transpose()
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn pi_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        profile: &PiProfile,
        before: Option<&Value>,
    ) -> Result<Value, String> {
        let key = Self::key(AppId::Pi, id)?;
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
        obj.insert("pi".into(), serde_json::to_value(profile).map_err(|_| "invalid profile")?);
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

impl AppProviderStore {
    #[cfg(target_os = "macos")]
    pub(crate) fn pi_usage_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        script: &vw_api_types::app_providers::pi::PiUsageScript,
    ) -> Result<(), String> {
        if script.auto_query_interval.is_some_and(|n| n > 1440) {
            return Err("invalid usage interval".into());
        }
        let key = Self::key(AppId::Pi, id)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "provider storage unavailable")?
            .ok_or("provider not found")?;
        let mut row: Value = serde_json::from_str(&raw).map_err(|_| "invalid provider record")?;
        let obj = row.as_object_mut().ok_or("invalid provider record")?;
        let meta = obj
            .entry("meta")
            .or_insert(json!({}))
            .as_object_mut()
            .ok_or("invalid provider metadata")?;
        meta.insert(
            "usageScript".into(),
            serde_json::to_value(script).map_err(|_| "invalid usage metadata")?,
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
