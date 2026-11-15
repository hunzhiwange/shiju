//! Compatibility forwarding for OMO requests.
use super::{AggregationOmoError, GatewayClient};
use vw_api_app_aggregation::{OmoApplyRequest, OmoDisableRequest, OmoState, OmoSlimState};
use vw_api_transport::id::ProjectId;

impl GatewayClient {
    /// Read the selected variant, including its current provider and opaque revision.
    pub async fn aggregation_omo_state(
        &self,
        host: &str,
        project: &ProjectId,
    ) -> Result<OmoState, AggregationOmoError> {
        gateway_aggregation_omo::GatewayAggregationOmoClient::new(self.transport.clone())
            .aggregation_omo_state(host, project).await
    }
    /// Apply a saved provider at the supplied revision; conflicts remain explicit.
    pub async fn aggregation_omo_apply(
        &self,
        request: &OmoApplyRequest,
    ) -> Result<OmoState, AggregationOmoError> {
        gateway_aggregation_omo::GatewayAggregationOmoClient::new(self.transport.clone())
            .aggregation_omo_apply(request).await
    }
    /// Disable only this variant at the supplied revision.
    pub async fn aggregation_omo_disable(
        &self,
        request: &OmoDisableRequest,
    ) -> Result<OmoState, AggregationOmoError> {
        gateway_aggregation_omo::GatewayAggregationOmoClient::new(self.transport.clone())
            .aggregation_omo_disable(request).await
    }
    /// Read the selected variant, including its current provider and opaque revision.
    pub async fn aggregation_omo_slim_state(
        &self,
        host: &str,
        project: &ProjectId,
    ) -> Result<OmoSlimState, AggregationOmoError> {
        gateway_aggregation_omo::GatewayAggregationOmoClient::new(self.transport.clone())
            .aggregation_omo_slim_state(host, project).await
    }
    /// Apply a saved provider at the supplied revision; conflicts remain explicit.
    pub async fn aggregation_omo_slim_apply(
        &self,
        request: &OmoApplyRequest,
    ) -> Result<OmoSlimState, AggregationOmoError> {
        gateway_aggregation_omo::GatewayAggregationOmoClient::new(self.transport.clone())
            .aggregation_omo_slim_apply(request).await
    }
    /// Disable only this variant at the supplied revision.
    pub async fn aggregation_omo_slim_disable(
        &self,
        request: &OmoDisableRequest,
    ) -> Result<OmoSlimState, AggregationOmoError> {
        gateway_aggregation_omo::GatewayAggregationOmoClient::new(self.transport.clone())
            .aggregation_omo_slim_disable(request).await
    }
}
