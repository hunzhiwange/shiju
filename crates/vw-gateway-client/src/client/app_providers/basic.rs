use super::{GatewayClient, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::basic::{ProviderBasic, ProviderBasicWrite},
};
impl GatewayClient {
    pub async fn app_provider_basic(&self, app: AppId, id: &str) -> Result<ProviderBasic, String> {
        self.get_json(&format!("{}/basic", provider_path(app, id)?), &[]).await
    }
    pub async fn app_provider_basic_write(
        &self,
        app: AppId,
        id: &str,
        value: &ProviderBasicWrite,
    ) -> Result<ProviderBasic, String> {
        let request = self
            .client
            .put(format!("{}{}/basic", self.endpoint.base_url(), provider_path(app, id)?))
            .json(value);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| "provider basic write transport failed")?;
        if !response.status().is_success() {
            return Err(format!("provider basic write failed: {}", response.status().as_u16()));
        }
        response.json().await.map_err(|_| "invalid provider basic response".into())
    }
}
