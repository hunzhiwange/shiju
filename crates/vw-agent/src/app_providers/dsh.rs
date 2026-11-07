//! Dsh supplier rows in the existing provider namespace with compare-and-exchange saves.
use super::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::dsh::DshProfile;
impl AppProviderStore {
    pub(crate) fn dsh_record(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<Option<Value>, String> {
        self.settings
            .get(tenant, NAMESPACE, &dsh_key(id)?)
            .map_err(|_| "provider storage unavailable")?
            .map(|s| serde_json::from_str(&s).map_err(|_| "invalid provider record".into()))
            .transpose()
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn dsh_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        profile: &DshProfile,
        before: Option<&Value>,
        metadata: Option<Value>,
    ) -> Result<Value, String> {
        let key = dsh_key(id)?;
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
        obj.insert("dsh".into(), serde_json::to_value(profile).map_err(|_| "invalid profile")?);
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

fn dsh_key(id: &str) -> Result<String, String> {
    if id.trim().is_empty()
        || id.len() > 128
        || id.chars().any(char::is_control)
        || matches!(id, "." | "..")
        || id.contains(['/', '\\'])
    {
        return Err("invalid Dsh provider identity".into());
    }
    Ok(format!("{}:{id}", serde_json::to_string(&AppId::Dsh).expect("AppId serialization")))
}
