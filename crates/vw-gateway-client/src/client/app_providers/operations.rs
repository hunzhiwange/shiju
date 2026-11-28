//! Fixed provider operation endpoints; raw worker errors never leave the host.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_app_aggregation::operations::*;
impl GatewayClient {
    pub async fn provider_operation_start(
        &self,
        request: &ProviderOperationRequest,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_operation_start(request, token).await
    }
    pub async fn provider_operation_get(
        &self,
        id: &str,
        host: &str,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_operation_get(id, host, token).await
    }
    pub async fn provider_operation_cancel(
        &self,
        id: &str,
        host: &str,
        token: &str,
    ) -> Result<ProviderOperationProgress, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_operation_cancel(id, host, token).await
    }
}
