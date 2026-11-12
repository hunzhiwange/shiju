//! Project-scoped Desktop resources; native configuration remains gateway-owned.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::agent_apps::aggregation::{
    AggregationExitRequest, AggregationPreview, AggregationPreviewRequest,
    AggregationPublishRequest, AggregationQuery, DesktopMappingChange, DesktopMappingView,
    DesktopProfileStatus,
};

use vw_api_types::app_providers::operations::ProviderOperationProgress;

impl GatewayClient {
    /// Read installation, applied mode and recovery without local file access.
    pub async fn aggregation_desktop_status(
        &self,
        scope: &AggregationQuery,
    ) -> Result<DesktopProfileStatus, AgentAppsSettingsError> {
        let request = self
            .native_transport()
            .map_err(AgentAppsSettingsError::Transport)?
            .get(format!(
                "{}/v1/agent-apps/claude-desktop/profile/status",
                self.endpoint.base_url()
            ))
            .query(scope);
        self.aggregation_desktop_response(request).await
    }

    /// Read the redacted mapping and its scoped revision without publishing.
    pub async fn aggregation_desktop_mapping(
        &self,
        provider_id: &str,
        scope: &AggregationQuery,
    ) -> Result<DesktopMappingView, AgentAppsSettingsError> {
        let url = self.aggregation_desktop_mapping_url(provider_id, "form/read")?;
        self.aggregation_desktop_response(
            self.native_transport()
                .map_err(AgentAppsSettingsError::Transport)?
                .post(url)
                .json(scope),
        )
        .await
    }

    /// Save exactly once at the observed revision. A transport error is never replayed.
    pub async fn aggregation_desktop_mapping_save(
        &self,
        provider_id: &str,
        change: &DesktopMappingChange,
    ) -> Result<DesktopMappingView, AgentAppsSettingsError> {
        let url = self.aggregation_desktop_mapping_url(provider_id, "model-mapping")?;
        self.aggregation_desktop_response(
            self.native_transport()
                .map_err(AgentAppsSettingsError::Transport)?
                .put(url)
                .json(change),
        )
        .await
    }

    /// Preview the Desktop catalog on the gateway; never issue a publish implicitly.
    pub async fn aggregation_desktop_preview(
        &self,
        intent: &AggregationPreviewRequest,
    ) -> Result<AggregationPreview, AgentAppsSettingsError> {
        self.aggregation_desktop_action("preview", intent).await
    }

    /// Submit a confirmed, revision-bound preview once. Poll the returned operation
    /// with `provider_aggregation_operation_get`; acceptance is not publication.
    pub async fn aggregation_desktop_publish(
        &self,
        intent: &AggregationPublishRequest,
    ) -> Result<ProviderOperationProgress, AgentAppsSettingsError> {
        self.aggregation_desktop_action("publish", intent).await
    }

    /// Request restoration using gateway-owned state; do not retry an ambiguous write.
    /// The returned operation retains its typed outcome, including recovery failures.
    pub async fn aggregation_desktop_exit(
        &self,
        intent: &AggregationExitRequest,
    ) -> Result<ProviderOperationProgress, AgentAppsSettingsError> {
        self.aggregation_desktop_action("exit", intent).await
    }

    async fn aggregation_desktop_action<B: Serialize, T: DeserializeOwned>(
        &self,
        action: &str,
        intent: &B,
    ) -> Result<T, AgentAppsSettingsError> {
        let url = format!(
            "{}/v1/agent-apps/claude-desktop/aggregation/{action}",
            self.endpoint.base_url()
        );
        self.aggregation_desktop_response(
            self.native_transport()
                .map_err(AgentAppsSettingsError::Transport)?
                .post(url)
                .json(intent),
        )
        .await
    }

    fn aggregation_desktop_mapping_url(
        &self,
        provider_id: &str,
        resource: &str,
    ) -> Result<String, AgentAppsSettingsError> {
        if provider_id.is_empty()
            || provider_id.len() > 128
            || !provider_id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
        {
            return Err(AgentAppsSettingsError::Transport("invalid provider identity".into()));
        }
        Ok(format!(
            "{}/v1/app-providers/claude-desktop/{provider_id}/{resource}",
            self.endpoint.base_url()
        ))
    }

    async fn aggregation_desktop_response<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, AgentAppsSettingsError> {
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|_| AgentAppsSettingsError::Transport("Desktop request failed".into()))?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| {
            AgentAppsSettingsError::Transport("Desktop response unavailable".into())
        })?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
            return Err(AgentAppsSettingsError::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| AgentAppsSettingsError::Decode("invalid Desktop response".into()))
    }
}
