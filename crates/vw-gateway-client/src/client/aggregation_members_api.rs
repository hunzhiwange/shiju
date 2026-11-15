//! Compatibility forwarding for aggregation requests.
use super::{AggregationApiError, GatewayClient};
use vw_api_app_aggregation::*;
use vw_api_app_identity::AppId;
impl GatewayClient {
    pub async fn aggregation_state(
        &self,
        app: AppId,
        scope: &AggregationQuery,
    ) -> Result<AggregationState, AggregationApiError> {
        gateway_aggregation::GatewayAggregationClient::new(self.transport.clone())
            .aggregation_state(app, scope).await
    }
    pub async fn aggregation_preview(
        &self,
        app: AppId,
        body: &AggregationPreviewRequest,
    ) -> Result<AggregationPreview, AggregationApiError> {
        gateway_aggregation::GatewayAggregationClient::new(self.transport.clone())
            .aggregation_preview(app, body).await
    }
    pub async fn aggregation_member_set(
        &self,
        app: AppId,
        provider_id: &str,
        body: &AggregationMemberRequest,
    ) -> Result<AggregationState, AggregationApiError> {
        gateway_aggregation::GatewayAggregationClient::new(self.transport.clone())
            .aggregation_member_set(app, provider_id, body).await
    }
    pub async fn aggregation_default_set(
        &self,
        app: AppId,
        body: &AggregationDefaultRequest,
    ) -> Result<AggregationState, AggregationApiError> {
        gateway_aggregation::GatewayAggregationClient::new(self.transport.clone())
            .aggregation_default_set(app, body).await
    }
}
