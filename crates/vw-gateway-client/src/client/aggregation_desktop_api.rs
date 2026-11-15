//! Project-scoped Desktop resources; native configuration remains gateway-owned.
use super::{AgentAppsSettingsError, GatewayClient};
use vw_api_app_aggregation::AggregationExitRequest;
use vw_api_app_aggregation::AggregationPreview;
use vw_api_app_aggregation::AggregationPreviewRequest;
use vw_api_app_aggregation::AggregationPublishRequest;
use vw_api_app_aggregation::AggregationQuery;
use vw_api_app_aggregation::DesktopMappingChange;
use vw_api_app_aggregation::DesktopMappingView;
use vw_api_app_aggregation::DesktopProfileStatus;

use vw_api_app_aggregation::operations::ProviderOperationProgress;

impl GatewayClient {
    /// Read installation, applied mode and recovery without local file access.
    pub async fn aggregation_desktop_status(
        &self,
        scope: &AggregationQuery,
    ) -> Result<DesktopProfileStatus, AgentAppsSettingsError> {
        gateway_aggregation_desktop::GatewayAggregationDesktopClient::new(self.transport.clone())
            .aggregation_desktop_status(scope).await
    }

    /// Read the redacted mapping and its scoped revision without publishing.
    pub async fn aggregation_desktop_mapping(
        &self,
        provider_id: &str,
        scope: &AggregationQuery,
    ) -> Result<DesktopMappingView, AgentAppsSettingsError> {
        gateway_aggregation_desktop::GatewayAggregationDesktopClient::new(self.transport.clone())
            .aggregation_desktop_mapping(provider_id, scope).await
    }

    /// Save exactly once at the observed revision. A transport error is never replayed.
    pub async fn aggregation_desktop_mapping_save(
        &self,
        provider_id: &str,
        change: &DesktopMappingChange,
    ) -> Result<DesktopMappingView, AgentAppsSettingsError> {
        gateway_aggregation_desktop::GatewayAggregationDesktopClient::new(self.transport.clone())
            .aggregation_desktop_mapping_save(provider_id, change).await
    }

    /// Preview the Desktop catalog on the gateway; never issue a publish implicitly.
    pub async fn aggregation_desktop_preview(
        &self,
        intent: &AggregationPreviewRequest,
    ) -> Result<AggregationPreview, AgentAppsSettingsError> {
        gateway_aggregation_desktop::GatewayAggregationDesktopClient::new(self.transport.clone())
            .aggregation_desktop_preview(intent).await
    }

    /// Submit a confirmed, revision-bound preview once. Poll the returned operation
    /// with `provider_aggregation_operation_get`; acceptance is not publication.
    pub async fn aggregation_desktop_publish(
        &self,
        intent: &AggregationPublishRequest,
    ) -> Result<ProviderOperationProgress, AgentAppsSettingsError> {
        gateway_aggregation_desktop::GatewayAggregationDesktopClient::new(self.transport.clone())
            .aggregation_desktop_publish(intent).await
    }

    /// Request restoration using gateway-owned state; do not retry an ambiguous write.
    /// The returned operation retains its typed outcome, including recovery failures.
    pub async fn aggregation_desktop_exit(
        &self,
        intent: &AggregationExitRequest,
    ) -> Result<ProviderOperationProgress, AgentAppsSettingsError> {
        gateway_aggregation_desktop::GatewayAggregationDesktopClient::new(self.transport.clone())
            .aggregation_desktop_exit(intent).await
    }

}
