//! Agy rows share the existing tenant store and conditional update contract.
use super::*;
use serde_json::{Value, json};
use vw_api_types::app_providers::agy::AgyProfile;
impl AppProviderStore {
    pub(crate) fn agy_record(&self, t: &TenantScope, id: &str) -> Result<Option<Value>, String> {
        self.settings
            .get(t, NAMESPACE, &Self::key(AppId::Agy, id)?)
            .map_err(|_| "provider storage unavailable")?
            .map(|s| serde_json::from_str(&s).map_err(|_| "invalid provider record".into()))
            .transpose()
    }
    #[cfg(any(test, target_os = "macos"))]
    pub(crate) fn agy_profile(&self, t: &TenantScope, id: &str) -> Result<AgyProfile, String> {
        let row = self.agy_record(t, id)?.ok_or("application provider not found")?;
        #[cfg(target_os = "macos")]
        if super::agy_binding::has_agy_binding(&row) {
            return Err("Agy bound model discovery requires an authorized account".into());
        }
        profile_from_record(&row)
    }

    pub(crate) fn agy_save(
        &self,
        t: &TenantScope,
        id: &str,
        p: &AgyProfile,
    ) -> Result<AppProviderSummary, String> {
        self.agy_save_inner(t, id, p, false)
    }
    #[cfg(target_os = "macos")]
    pub(crate) fn agy_insert(
        &self,
        t: &TenantScope,
        id: &str,
        p: &AgyProfile,
    ) -> Result<AppProviderSummary, String> {
        self.agy_save_inner(t, id, p, true)
    }
    fn agy_save_inner(
        &self,
        t: &TenantScope,
        id: &str,
        p: &AgyProfile,
        create: bool,
    ) -> Result<AppProviderSummary, String> {
        #[cfg(target_os = "macos")]
        vw_agent_apps::providers::agy::validate(p)?;
        let key = Self::key(AppId::Agy, id)?;
        let raw =
            self.settings.get(t, NAMESPACE, &key).map_err(|_| "provider storage unavailable")?;
        if create && raw.is_some() {
            return Err("application provider already exists".into());
        }
        let mut row: Value = raw
            .as_ref()
            .map(|s| serde_json::from_str(s).map_err(|_| "invalid provider record"))
            .transpose()?
            .unwrap_or(json!({}));
        if row.pointer("/meta/authBinding").is_some()
            || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
        {
            return Err("Agy managed binding is unsupported".into());
        }
        #[cfg(target_os = "macos")]
        super::agy_binding::check_profile_edit(&row, p)?;
        let obj = row.as_object_mut().ok_or("invalid provider record")?;
        for k in ["apiKey", "baseUrl", "protocol", "model", "requestOptions"] {
            obj.remove(k);
        }
        obj.insert("name".into(), json!(p.name));
        obj.insert("agy".into(), serde_json::to_value(p).map_err(|_| "invalid profile")?);
        obj.insert("settingsConfig".into(),json!({"env":{"GEMINI_API_KEY":p.api_key,"GOOGLE_GEMINI_BASE_URL":p.base_url},"config":{"model":{"name":p.model}}}));
        let value = row.to_string();
        let ok = match raw {
            Some(raw) => self.settings.compare_exchange(t, NAMESPACE, &key, &raw, &value),
            None => self.settings.insert_if_absent(t, NAMESPACE, &key, &value),
        }
        .map_err(|_| "provider storage unavailable")?;
        if !ok {
            return Err("application provider changed during update".into());
        }
        Ok(AppProviderSummary {
            app_id: AppId::Agy,
            id: id.into(),
            name: p.name.clone(),
            configured: true,
        })
    }
}

#[cfg(target_os = "macos")]
impl AppProviderStore {
    pub(crate) fn prepare_agy_launch(
        &self,
        scope: crate::agent_apps::auth_accounts::AuthAccountScope<'_>,
        native: vw_agent_apps::auth::agy_account::AgyNativeTarget<'_>,
        id: &str,
        private_root: &Path,
        material: vw_agent_apps::apps::launch::request::LaunchMaterial,
    ) -> Result<ProviderLaunch, String> {
        let before = self.agy_record(scope.tenant, id)?.ok_or("application provider not found")?;
        if before.get("requestOptions").is_some_and(|v| !v.is_null()) {
            return Err("Agy request overrides are unsupported".into());
        }
        let profile = profile_from_record(&before)?;
        let account =
            super::agy_binding::account(&before, scope).map_err(|_| "Agy binding unavailable")?;
        let unchanged = || matches!(self.agy_record(scope.tenant,id),Ok(Some(row)) if row==before);
        let mut launch = vw_agent_apps::providers::agy::prepare_checked(
            &vw_agent_apps::providers::agy::Scope {
                home: native.home,
                private_root,
                host: scope.host,
                tenant: scope.tenant,
                provider: id,
            },
            &profile,
            material,
            unchanged,
        )?;
        if let Some(account) = account {
            let lease = vw_agent_apps::auth::agy_account::checkout_provider_account(
                scope, &account, native, unchanged,
            )
            .map_err(|_| "Agy bound account unavailable")?;
            launch.hold_agy_account(lease);
        }
        Ok(launch)
    }
}

#[cfg(any(test, target_os = "macos"))]
pub(super) fn profile_from_record(row: &Value) -> Result<AgyProfile, String> {
    if row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
    {
        return Err("Agy managed binding is unsupported".into());
    }
    if let Some(p) = row.get("agy") {
        return serde_json::from_value(p.clone()).map_err(|_| "invalid Agy profile".into());
    }
    // Frozen source: only explicit category/partner selects native login; names never do.
    if row.get("category").and_then(Value::as_str) == Some("official")
        || row.pointer("/meta/partnerPromotionKey").and_then(Value::as_str)
            == Some("google-official")
    {
        let model = row.get("model").or_else(|| row.pointer("/settingsConfig/config/model/name"));
        let model = match model {
            Some(Value::Null) | None => None,
            Some(Value::String(v)) => Some(v.clone()),
            _ => return Err("invalid Agy model".into()),
        };
        return Ok(AgyProfile {
            name: row.get("name").and_then(Value::as_str).ok_or("invalid Agy name")?.into(),
            model,
            api_key: None,
            base_url: None,
        });
    }
    let p = vw_agent_apps::providers::update::profile(row)?;
    validate_profile(AppId::Agy, &p)?;
    Ok(AgyProfile {
        name: p.name,
        model: Some(p.model),
        api_key: Some(p.api_key),
        base_url: Some(p.base_url),
    })
}
