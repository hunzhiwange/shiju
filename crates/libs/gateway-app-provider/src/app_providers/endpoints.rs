use super::{GatewayAppProviderClient, provider_path};
use vw_api_app_provider_edit::endpoints::AppProviderEndpoint;
use vw_api_app_provider_edit::endpoints::AppProviderEndpointChange;
use vw_api_app_identity::{AppId};
impl GatewayAppProviderClient {
    pub async fn app_provider_endpoint_change(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderEndpointChange,
    ) -> Result<Vec<AppProviderEndpoint>, String> {
        let builder = self.transport.client()
            .post(format!("{}{}/endpoints", self.transport.endpoint().base_url(), provider_path(app, id)?))
            .json(request);
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
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
        let builder = self.transport.client().get(format!(
            "{}{}/endpoints",
            self.transport.endpoint().base_url(),
            provider_path(app, id)?
        ));
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
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
