//! Codex settings and client hints stay gateway-owned; writes are never replayed.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::de::DeserializeOwned;
use vw_api_types::agent_apps::aggregation::{
    AggregationQuery, CodexAggregationSettings, CodexClientsAckRequest, CodexClientsState,
    CodexRestartRequest, CodexSettingsRequest,
};
use vw_api_types::app_providers::operations::ProviderOperationProgress;

impl GatewayClient {
    /// Read requested and effective settings separately at the selected scope.
    pub async fn aggregation_codex_settings(
        &self,
        scope: &AggregationQuery,
    ) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
        self.aggregation_codex_response(
            self.dynamic_host_client.get(self.aggregation_codex_url("settings")).query(scope),
            reqwest::StatusCode::OK,
        )
        .await
    }

    /// Save intent at the observed settings revision without restarting clients.
    pub async fn aggregation_codex_settings_save(
        &self,
        intent: &CodexSettingsRequest,
    ) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
        self.aggregation_codex_response(
            self.dynamic_host_client.put(self.aggregation_codex_url("settings")).json(intent),
            reqwest::StatusCode::OK,
        )
        .await
    }

    /// Read bounded stale-client hints; `auth` is an inference, not process memory.
    pub async fn aggregation_codex_clients(
        &self,
        scope: &AggregationQuery,
    ) -> Result<CodexClientsState, AgentAppsSettingsError> {
        self.aggregation_codex_response(
            self.dynamic_host_client.get(self.aggregation_codex_url("clients")).query(scope),
            reqwest::StatusCode::OK,
        )
        .await
    }

    /// Acknowledge this clients revision only; acknowledgement does not restart clients.
    pub async fn aggregation_codex_ack(
        &self,
        intent: &CodexClientsAckRequest,
    ) -> Result<CodexClientsState, AgentAppsSettingsError> {
        self.aggregation_codex_response(
            self.dynamic_host_client.post(self.aggregation_codex_url("ack")).json(intent),
            reqwest::StatusCode::OK,
        )
        .await
    }

    /// Submit explicit confirmation and the clients revision for a managed daemon only.
    /// The gateway independently authorizes restart and rejects unconfirmed intent.
    /// Acceptance is not success: use `provider_aggregation_operation_get` and
    /// `provider_aggregation_operation_cancel` with the same host/project scope.
    pub async fn aggregation_codex_restart(
        &self,
        intent: &CodexRestartRequest,
    ) -> Result<ProviderOperationProgress, AgentAppsSettingsError> {
        self.aggregation_codex_response(
            self.dynamic_host_client.post(self.aggregation_codex_url("restart")).json(intent),
            reqwest::StatusCode::ACCEPTED,
        )
        .await
    }

    fn aggregation_codex_url(&self, resource: &str) -> String {
        format!("{}/v1/agent-apps/codex/aggregation/codex/{resource}", self.endpoint.base_url())
    }

    async fn aggregation_codex_response<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        expected: reqwest::StatusCode,
    ) -> Result<T, AgentAppsSettingsError> {
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| AgentAppsSettingsError::Transport("Codex request failed".into()))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|_| AgentAppsSettingsError::Transport("Codex response unavailable".into()))?;
        if status != expected {
            return Err(AgentAppsSettingsError::Http {
                status: status.as_u16(),
                body: serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
            });
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| AgentAppsSettingsError::Decode("invalid Codex response".into()))
    }
}
