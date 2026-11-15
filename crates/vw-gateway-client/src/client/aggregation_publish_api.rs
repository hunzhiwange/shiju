//! Compatibility forwarding for aggregation requests.
use super::{AggregationApiError, GatewayClient};
use vw_api_app_aggregation::*;
use vw_api_app_identity::AppId;
use vw_api_app_aggregation::operations::ProviderOperationProgress;
impl GatewayClient {
    pub async fn aggregation_publish(
        &self,
        app: AppId,
        body: &AggregationPublishRequest,
    ) -> Result<ProviderOperationProgress, AggregationApiError> {
        gateway_aggregation::GatewayAggregationClient::new(self.transport.clone())
            .aggregation_publish(app, body).await
    }
    pub async fn aggregation_exit(
        &self,
        app: AppId,
        body: &AggregationExitRequest,
    ) -> Result<ProviderOperationProgress, AggregationApiError> {
        gateway_aggregation::GatewayAggregationClient::new(self.transport.clone())
            .aggregation_exit(app, body).await
    }
}
