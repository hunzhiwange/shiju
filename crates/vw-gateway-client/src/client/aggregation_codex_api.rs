//! Codex settings and client hints stay gateway-owned; writes are never replayed.
use super::{AgentAppsSettingsError, GatewayClient};
use vw_api_app_aggregation::AggregationQuery;
use vw_api_app_aggregation::CodexAggregationSettings;
use vw_api_app_aggregation::CodexClientsAckRequest;
use vw_api_app_aggregation::CodexClientsState;
use vw_api_app_aggregation::CodexRestartRequest;
use vw_api_app_aggregation::CodexSettingsRequest;
use vw_api_app_aggregation::operations::ProviderOperationProgress;

impl GatewayClient {
    /// Read requested and effective settings separately at the selected scope.
    pub async fn aggregation_codex_settings(
        &self,
        scope: &AggregationQuery,
    ) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
        gateway_aggregation_codex::GatewayAggregationCodexClient::new(self.transport.clone())
            .aggregation_codex_settings(scope).await
    }

    /// Save intent at the observed settings revision without restarting clients.
    pub async fn aggregation_codex_settings_save(
        &self,
        intent: &CodexSettingsRequest,
    ) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
        gateway_aggregation_codex::GatewayAggregationCodexClient::new(self.transport.clone())
            .aggregation_codex_settings_save(intent).await
    }

    /// Read bounded stale-client hints; `auth` is an inference, not process memory.
    pub async fn aggregation_codex_clients(
        &self,
        scope: &AggregationQuery,
    ) -> Result<CodexClientsState, AgentAppsSettingsError> {
        gateway_aggregation_codex::GatewayAggregationCodexClient::new(self.transport.clone())
            .aggregation_codex_clients(scope).await
    }

    /// Acknowledge this clients revision only; acknowledgement does not restart clients.
    pub async fn aggregation_codex_ack(
        &self,
        intent: &CodexClientsAckRequest,
    ) -> Result<CodexClientsState, AgentAppsSettingsError> {
        gateway_aggregation_codex::GatewayAggregationCodexClient::new(self.transport.clone())
            .aggregation_codex_ack(intent).await
    }

    /// Submit explicit confirmation and the clients revision for a managed daemon only.
    /// The gateway independently authorizes restart and rejects unconfirmed intent.
    /// Acceptance is not success: use `provider_aggregation_operation_get` and
    /// `provider_aggregation_operation_cancel` with the same host/project scope.
    pub async fn aggregation_codex_restart(
        &self,
        intent: &CodexRestartRequest,
    ) -> Result<ProviderOperationProgress, AgentAppsSettingsError> {
        gateway_aggregation_codex::GatewayAggregationCodexClient::new(self.transport.clone())
            .aggregation_codex_restart(intent).await
    }

}
