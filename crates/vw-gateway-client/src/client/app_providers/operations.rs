//! Fixed provider operation endpoints; raw worker errors never leave the host.
use super::super::agent_apps_read_api::AgentAppsReadError as Error;
use super::GatewayClient;
use vw_api_types::{agent_apps::aggregation::AggregationQuery, app_providers::operations::*};
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
    /// Query the original project-owned aggregation operation using endpoint Bearer auth.
    /// A failed outcome remains a typed operation result; never replay the submit here.
    pub async fn provider_aggregation_operation_get(
        &self,
        id: &str,
        scope: &AggregationQuery,
    ) -> Result<ProviderOperationProgress, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_aggregation_operation_get(id, scope).await
    }
    /// Request cancellation once; the returned snapshot is authoritative even if
    /// cancellation was requested after the irreversible publication boundary.
    pub async fn provider_aggregation_operation_cancel(
        &self,
        id: &str,
        scope: &AggregationQuery,
    ) -> Result<ProviderOperationProgress, Error> {
        gateway_app_provider::GatewayAppProviderClient::new(self.transport.clone())
            .provider_aggregation_operation_cancel(id, scope).await
    }
}
