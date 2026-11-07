//! Claude records use the established tenant namespace and compare/exchange owner.
use super::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::claude::ClaudeProfile;
impl AppProviderStore {
    pub(crate) fn claude_record(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<Option<Value>, String> {
        self.settings
            .get(tenant, NAMESPACE, &Self::key(AppId::Claude, id)?)
            .map_err(|_| "application provider storage unavailable".to_string())?
            .map(|raw| {
                serde_json::from_str(&raw)
                    .map_err(|_| "invalid stored application provider profile".into())
            })
            .transpose()
    }
    pub(crate) fn claude_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        profile: &ClaudeProfile,
        before: Option<&Value>,
        create: bool,
    ) -> Result<Value, String> {
        self.claude_save_with_metadata(tenant, id, profile, before, create, None)
    }

    pub(crate) fn claude_save_with_metadata(
        &self,
        tenant: &TenantScope,
        id: &str,
        profile: &ClaudeProfile,
        before: Option<&Value>,
        create: bool,
        metadata: Option<Value>,
    ) -> Result<Value, String> {
        let settings = vw_agent_apps::providers::claude::settings(profile)
            .map_err(|_| "invalid application provider profile")?;
        let key = Self::key(AppId::Claude, id)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?;
        let stored: Option<Value> = raw
            .as_ref()
            .map(|s| {
                serde_json::from_str(s).map_err(|_| "invalid stored application provider profile")
            })
            .transpose()?;
        if stored.as_ref() != before {
            return Err("application provider changed during update".into());
        }
        if create && stored.is_some() {
            return Err("application provider already exists".into());
        }
        let preserve_category = metadata.is_some();
        let mut row = metadata.or(stored).unwrap_or(json!({}));
        let object = row.as_object_mut().ok_or("invalid stored application provider profile")?;
        // Remove legacy credentials as well as provider-owned settings, retain metadata/global fields.
        for key in ["apiKey", "baseUrl", "protocol", "model"] {
            object.remove(key);
        }
        if let Some(meta) = object.get_mut("meta").and_then(Value::as_object_mut) {
            meta.remove("authBinding");
            meta.remove("claudeAccountId");
        }
        object.insert("name".into(), json!(profile.name));
        if !preserve_category
            || !object
                .get("category")
                .and_then(Value::as_str)
                .is_some_and(|value| !matches!(value, "official" | "custom"))
        {
            object.insert(
                "category".into(),
                json!(if matches!(
                    profile.config,
                    vw_api_types::app_providers::claude::ClaudeConfig::Official { .. }
                ) {
                    "official"
                } else {
                    "custom"
                }),
            );
        }
        object.insert(
            "claude".into(),
            serde_json::to_value(profile).map_err(|_| "invalid application provider profile")?,
        );
        let previous = object.get("settingsConfig").cloned().unwrap_or(json!({}));
        // Reuse projection to retain saved global fields; only the new owned fields are replaced.
        let bytes = vw_agent_apps::providers::claude::patch(
            Some(previous.to_string().as_bytes()),
            Some(&previous),
            &settings,
        )
        .map_err(|_| "invalid application provider profile")?;
        object.insert(
            "settingsConfig".into(),
            serde_json::from_slice(&bytes).map_err(|_| "invalid application provider profile")?,
        );
        vw_agent_apps::providers::request_options::read(AppId::Claude, &row)?;
        let after = row.to_string();
        let saved = match raw {
            Some(raw) => self.settings.compare_exchange(tenant, NAMESPACE, &key, &raw, &after),
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &after),
        }
        .map_err(|_| "application provider storage unavailable")?;
        if !saved {
            return Err("application provider changed during update".into());
        }
        Ok(row)
    }
}

#[cfg(target_os = "macos")]
impl AppProviderStore {
    pub(crate) fn claude_accounts(
        &self,
        root: &std::path::Path,
    ) -> Result<crate::agent_apps::auth_accounts::AuthAccountStore, String> {
        crate::agent_apps::auth_accounts::AuthAccountStore::new(self.database.clone(), root)
    }
}
