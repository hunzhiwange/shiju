//! Project-scoped Desktop resources; native configuration remains gateway-owned.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::de::DeserializeOwned;
use vw_api_types::agent_apps::aggregation::{
    AggregationQuery, DesktopMappingChange, DesktopMappingView, DesktopProfileStatus,
};

impl GatewayClient {
    /// Read installation, applied mode and recovery without local file access.
    pub async fn aggregation_desktop_status(
        &self,
        scope: &AggregationQuery,
    ) -> Result<DesktopProfileStatus, AgentAppsSettingsError> {
        let request = self
            .dynamic_host_client
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
        self.aggregation_desktop_response(self.dynamic_host_client.post(url).json(scope)).await
    }

    /// Save exactly once at the observed revision. A transport error is never replayed.
    pub async fn aggregation_desktop_mapping_save(
        &self,
        provider_id: &str,
        change: &DesktopMappingChange,
    ) -> Result<DesktopMappingView, AgentAppsSettingsError> {
        let url = self.aggregation_desktop_mapping_url(provider_id, "model-mapping")?;
        self.aggregation_desktop_response(self.dynamic_host_client.put(url).json(change)).await
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
