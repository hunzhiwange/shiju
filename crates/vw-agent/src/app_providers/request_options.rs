use super::*;
use vw_api_types::app_providers::{
    request_options::RequestOptionsView, validation::ProviderRequestOptions,
};
impl AppProviderStore {
    pub fn request_options_view(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<RequestOptionsView, String> {
        let raw = self
            .settings
            .get(tenant, NAMESPACE, &Self::key(app, id)?)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let row = serde_json::from_str(&raw)
            .map_err(|_| "invalid stored application provider profile")?;
        vw_agent_apps::providers::request_options::view(app, &row)
    }
    pub fn request_options_save(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
        options: &ProviderRequestOptions,
    ) -> Result<RequestOptionsView, String> {
        let key = Self::key(app, id)?;
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let row = serde_json::from_str(&before)
            .map_err(|_| "invalid stored application provider profile")?;
        let updated = vw_agent_apps::providers::request_options::update(app, &row, options)?;
        let view = vw_agent_apps::providers::request_options::view(app, &updated)?;
        if !self
            .settings
            .compare_exchange(tenant, NAMESPACE, &key, &before, &updated.to_string())
            .map_err(|_| "application provider storage unavailable")?
        {
            return Err("application provider changed during update".into());
        }
        Ok(view)
    }
}
