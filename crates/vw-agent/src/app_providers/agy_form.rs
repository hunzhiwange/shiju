//! Agy catalog editor: public revision, write-only credentials, no live activation.
use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use vw_api_types::app_providers::{agy::*, create::ProviderSecretWrite};

fn view(row: &Value) -> Result<AgyFormView, String> {
    let profile = super::agy::profile_from_record(row)?;
    vw_agent_apps::providers::agy::validate(&profile)?;
    if row.get("requestOptions").is_some_and(|v| !v.is_null()) {
        return Err("Agy request overrides are unsupported".into());
    }
    let fields = vw_agent_apps::providers::basic::read(row)?;
    let has_saved_key = profile.api_key.is_some();
    let revision = hex::encode(Sha256::digest(
        serde_json::to_vec(&(&fields, &profile.model, &profile.base_url, has_saved_key))
            .map_err(|_| "invalid profile")?,
    ));
    Ok(AgyFormView {
        fields,
        model: profile.model,
        base_url: profile.base_url,
        has_saved_key,
        revision,
    })
}
impl AppProviderStore {
    pub(crate) fn agy_form_read(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<AgyFormView, String> {
        view(&self.agy_record(tenant, id)?.ok_or("application provider not found")?)
    }
    pub(crate) fn agy_form_write(
        &self,
        tenant: &TenantScope,
        id: &str,
        body: &AgyFormWrite,
        create: bool,
    ) -> Result<AgySaved, String> {
        let key = Self::key(AppId::Agy, id)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "provider storage unavailable")?;
        if create != raw.is_none() {
            return Err("provider changed".into());
        }
        let before: Value = raw
            .as_ref()
            .map(|s| serde_json::from_str(s))
            .transpose()
            .map_err(|_| "invalid profile")?
            .unwrap_or(json!({}));
        let previous = if raw.is_some() {
            let public = view(&before)?;
            if body.revision.as_deref() != Some(public.revision.as_str()) {
                return Err("provider changed".into());
            }
            Some(super::agy::profile_from_record(&before)?)
        } else {
            if body.revision.is_some() {
                return Err("provider changed".into());
            }
            None
        };
        let api_key = match &body.api_key {
            ProviderSecretWrite::Keep => previous.as_ref().and_then(|p| p.api_key.clone()),
            ProviderSecretWrite::Replace { value } => {
                if value.trim().eq_ignore_ascii_case("[redacted]")
                    || value.trim().eq_ignore_ascii_case("<redacted>")
                {
                    return Err("invalid Agy profile".into());
                }
                Some(value.clone())
            }
            ProviderSecretWrite::Clear => None,
        };
        // A retained credential must never be redirected to a new endpoint.
        if matches!(body.api_key, ProviderSecretWrite::Keep)
            && api_key.is_some()
            && previous.as_ref().is_some_and(|p| p.base_url != body.base_url)
        {
            return Err("invalid Agy endpoint change".into());
        }
        let profile = AgyProfile {
            name: body.fields.name.clone(),
            model: body.model.clone(),
            base_url: body.base_url.clone(),
            api_key,
        };
        vw_agent_apps::providers::agy::validate(&profile)?;
        super::agy_binding::check_profile_edit(&before, &profile)?;
        let mut next = vw_agent_apps::providers::basic::apply(
            AppId::Agy,
            &before,
            &body.fields,
            &ProviderSecretWrite::Keep,
        )?;
        for key in ["apiKey", "baseUrl", "protocol", "model", "requestOptions"] {
            next.as_object_mut().ok_or("invalid profile")?.remove(key);
        }
        // Preserve unrelated metadata and settings; only these profile fields are owned here.
        let encoded = serde_json::to_value(&profile).map_err(|_| "invalid profile")?;
        let node = next.as_object_mut().ok_or("invalid profile")?.entry("agy").or_insert(json!({}));
        let object = node.as_object_mut().ok_or("invalid profile")?;
        for (key, value) in encoded.as_object().ok_or("invalid profile")? {
            object.insert(key.clone(), value.clone());
        }
        let settings = next
            .as_object_mut()
            .ok_or("invalid profile")?
            .entry("settingsConfig")
            .or_insert(json!({}));
        let settings = settings.as_object_mut().ok_or("invalid profile")?;
        let env =
            settings.entry("env").or_insert(json!({})).as_object_mut().ok_or("invalid profile")?;
        env.insert("GEMINI_API_KEY".into(), json!(profile.api_key));
        env.insert("GOOGLE_GEMINI_BASE_URL".into(), json!(profile.base_url));
        let config = settings
            .entry("config")
            .or_insert(json!({}))
            .as_object_mut()
            .ok_or("invalid profile")?;
        config
            .entry("model")
            .or_insert(json!({}))
            .as_object_mut()
            .ok_or("invalid profile")?
            .insert("name".into(), json!(profile.model));
        let saved = match raw {
            Some(raw) => {
                self.settings.compare_exchange(tenant, NAMESPACE, &key, &raw, &next.to_string())
            }
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &next.to_string()),
        }
        .map_err(|_| "provider storage unavailable")?;
        if !saved {
            return Err("provider changed".into());
        }
        Ok(AgySaved { catalog_saved: true })
    }
}

impl AppProviderStore {
    /// Caller retains the exclusive catalog lease until this conditional deletion completes.
    pub(crate) fn agy_form_delete(&self, tenant: &TenantScope, id: &str) -> Result<(), String> {
        let key = Self::key(AppId::Agy, id)?;
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "provider storage unavailable")?
            .ok_or("application provider not found")?;
        let row: Value = serde_json::from_str(&raw).map_err(|_| "invalid profile")?;
        view(&row)?;
        if super::agy_binding::has_agy_binding(&row) {
            return Err("Agy provider is referenced".into());
        }
        // Unknown metadata can carry bindings; refuse rather than guess reference semantics.
        if row
            .get("meta")
            .is_some_and(|m| !m.is_null() && m.as_object().is_none_or(|m| !m.is_empty()))
        {
            return Err("Agy provider references unavailable".into());
        }
        if !self
            .settings
            .compare_delete(tenant, NAMESPACE, &key, &raw)
            .map_err(|_| "provider storage unavailable")?
        {
            return Err("provider changed".into());
        }
        Ok(())
    }
}
