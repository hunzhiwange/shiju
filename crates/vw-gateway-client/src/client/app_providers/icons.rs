use super::{GatewayClient, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::icons::{AppProviderIcon, AppProviderIconChange},
};
impl GatewayClient {
    pub async fn app_provider_icon(&self, app: AppId, id: &str) -> Result<AppProviderIcon, String> {
        let builder = self.client.get(format!(
            "{}{}/icon",
            self.endpoint.base_url(),
            provider_path(app, id)?
        ));
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "provider icon transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("provider icon request failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid provider icon response".into())
    }
    pub async fn app_provider_icon_change(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderIconChange,
    ) -> Result<AppProviderIcon, String> {
        let builder = self
            .client
            .put(format!("{}{}/icon", self.endpoint.base_url(), provider_path(app, id)?))
            .json(request);
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "provider icon transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("provider icon request failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid provider icon response".into())
    }
}
