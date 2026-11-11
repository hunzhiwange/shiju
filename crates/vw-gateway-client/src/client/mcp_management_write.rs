//! Typed catalog mutations on the selected gateway; native writes stay on its execution host.
use super::{AgentAppsSettingsError, GatewayClient};
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::mcp_management::{
    McpCatalogRecord, McpCreateRequest, McpDeleteRequest, McpDistributionResult, McpRecordId,
    McpUpdateRequest,
};

impl GatewayClient {
    /// Save a desired definition. Success does not imply native distribution or connection.
    pub async fn mcp_management_create(
        &self,
        request: &McpCreateRequest,
        token: &str,
    ) -> Result<McpCatalogRecord, AgentAppsSettingsError> {
        self.mcp_management_write(reqwest::Method::POST, None, request, token).await
    }

    /// Replace a connection at the expected revision, preserving display metadata.
    pub async fn mcp_management_update(
        &self,
        id: &McpRecordId,
        request: &McpUpdateRequest,
        token: &str,
    ) -> Result<McpCatalogRecord, AgentAppsSettingsError> {
        self.mcp_management_write(reqwest::Method::PUT, Some(id), request, token).await
    }

    /// Retire the expected revision, preserving each native cleanup outcome.
    pub async fn mcp_management_delete(
        &self,
        id: &McpRecordId,
        request: &McpDeleteRequest,
        token: &str,
    ) -> Result<McpDistributionResult, AgentAppsSettingsError> {
        self.mcp_management_write(reqwest::Method::DELETE, Some(id), request, token).await
    }

    async fn mcp_management_write<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        id: Option<&McpRecordId>,
        body: &B,
        token: &str,
    ) -> Result<T, AgentAppsSettingsError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, id, body, token);
            Err(AgentAppsSettingsError::Transport(
                "MCP catalog writes require native transport with redirects and retries disabled"
                    .into(),
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut url =
                reqwest::Url::parse(&format!("{}/v1/agent-apps/mcp", self.endpoint.base_url()))
                    .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            if let Some(id) = id {
                if id.0.is_empty() || matches!(id.0.as_str(), "." | "..") {
                    return Err(AgentAppsSettingsError::Transport(
                        "invalid MCP record identity".into(),
                    ));
                }
                url.path_segments_mut()
                    .map_err(|_| AgentAppsSettingsError::Transport("invalid gateway URL".into()))?
                    .push(&id.0);
            }
            let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                AgentAppsSettingsError::Transport("invalid application credential".into())
            })?;
            credential.set_sensitive(true);
            let request = self
                .dynamic_host_client
                .request(method, url)
                .header("x-agent-apps-token", credential)
                .json(body);
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            let status = response.status();
            let bytes = response
                .bytes()
                .await
                .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                    serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
                });
                return Err(AgentAppsSettingsError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes)
                .map_err(|err| AgentAppsSettingsError::Decode(err.to_string()))
        }
    }
}
