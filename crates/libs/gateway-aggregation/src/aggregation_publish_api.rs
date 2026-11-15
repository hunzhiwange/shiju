//! Submit aggregation writes once; the gateway owns publication and recovery.
use super::{AggregationApiError, GatewayAggregationClient};
use vw_api_app_aggregation::AggregationExitRequest;
use vw_api_app_aggregation::AggregationPublishRequest;
use vw_api_app_aggregation::operations::ProviderOperationProgress;
use vw_api_app_identity::{AppId};
impl GatewayAggregationClient {
    /// Submit the caller's confirmed preview once. Use the returned operation ID with
    /// `provider_aggregation_operation_get` or `provider_aggregation_operation_cancel`.
    /// On an ambiguous transport failure, read gateway state before any new intent.
    pub async fn aggregation_publish(
        &self,
        app: AppId,
        body: &AggregationPublishRequest,
    ) -> Result<ProviderOperationProgress, AggregationApiError> {
        self.aggregation_write(reqwest::Method::POST, app, &["publish"], body).await
    }
    /// Restore through the gateway's saved configuration, without replaying writes.
    /// Acceptance and cancellation requests do not imply restoration; the operation's
    /// phase and typed outcome retain the authoritative recovery state.
    pub async fn aggregation_exit(
        &self,
        app: AppId,
        body: &AggregationExitRequest,
    ) -> Result<ProviderOperationProgress, AggregationApiError> {
        self.aggregation_write(reqwest::Method::POST, app, &["exit"], body).await
    }
}
