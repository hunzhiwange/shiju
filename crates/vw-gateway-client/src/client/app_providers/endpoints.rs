use super::GatewayClient;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::endpoints::{AppProviderEndpoint, AppProviderEndpointChange},
};
use vw_api_app_identity::{AppId};
impl GatewayClient {
    pub async fn app_provider_endpoint_change(
        &self,
        app: AppId,
        id: &str,
        request: &AppProviderEndpointChange,
    ) -> Result<Vec<AppProviderEndpoint>, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_endpoint_change(app, id, request).await
    }
    pub async fn app_provider_endpoints(
        &self,
        app: AppId,
        id: &str,
    ) -> Result<Vec<AppProviderEndpoint>, String> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .app_provider_endpoints(app, id).await
    }
}
