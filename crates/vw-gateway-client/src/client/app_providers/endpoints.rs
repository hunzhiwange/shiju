use super::{GatewayClient, provider_path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::endpoints::{AppProviderEndpoint, AppProviderEndpointChange},
};
impl GatewayClient {
    pub async fn app_provider_endpoint_change(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderEndpointChange,
    ) -> Result<Vec<AppProviderEndpoint>, String> {
        let builder = self
            .client
            .post(format!("{}{}/endpoints", self.endpoint.base_url(), provider_path(app, id)?))
            .json(request);
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider endpoint transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider endpoint failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider endpoint response".into())
    }
    pub async fn app_provider_endpoints(
        &self,
        app: AppId,
        id: &str,
    ) -> Result<Vec<AppProviderEndpoint>, String> {
        let builder = self.client.get(format!(
            "{}{}/endpoints",
            self.endpoint.base_url(),
            provider_path(app, id)?
        ));
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|_| "application provider endpoint transport failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("application provider endpoint failed: {}", status.as_u16()));
        }
        response.json().await.map_err(|_| "invalid application provider endpoint response".into())
    }
}
