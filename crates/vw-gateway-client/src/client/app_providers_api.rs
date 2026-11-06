//! Tenant application-provider records; no relationship to model-provider auth APIs.
use super::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, AppProviderWrite},
};
impl GatewayClient {
    pub async fn app_provider_save(
        &self,
        app: AppId,
        id: &str,
        profile: &AppProviderWrite,
    ) -> Result<AppProviderSummary, String> {
        self.put_json(&provider_path(app, id)?, &[], profile).await
    }
    pub async fn app_provider_get(
        &self,
        app: AppId,
        id: &str,
    ) -> Result<AppProviderSummary, String> {
        self.get_json(&provider_path(app, id)?, &[]).await
    }
}
fn provider_path(app: AppId, id: &str) -> Result<String, String> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("invalid application provider identity".into());
    }
    let app = serde_json::to_string(&app).map_err(|_| "invalid application identity")?;
    Ok(format!("/v1/app-providers/{}/{id}", app.trim_matches('"')))
}
