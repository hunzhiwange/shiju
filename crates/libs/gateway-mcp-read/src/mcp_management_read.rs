//! Typed reads from the selected gateway's managed MCP catalog.
use super::{AgentAppsReadError, GatewayMcpReadClient};
use vw_api_mcp_management::{
    McpAppTarget, McpCatalogResponse, McpNativePreview, McpPresetsResponse,
};

impl GatewayMcpReadClient {
    /// Read committed references for one explicitly authorized inbound ACP session.
    pub async fn mcp_management_acp_usage(
        &self,
        query: &vw_api_mcp_management::McpAcpUsageQuery,
        token: &str,
    ) -> Result<vw_api_mcp_management::McpAcpUsage, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone()).agent_apps_read_request(
            "/v1/agent-apps/mcp/acp-mounts/query",
            &[
                ("targetHostId", query.target_host_id.clone()),
                ("projectId", query.project_id.as_ref().to_owned()),
                ("sessionId", query.session_id.as_ref().to_owned()),
            ],
            Some(token),
        )
        .await
    }

    /// Read public catalog records, desired bindings and native adapter capabilities.
    pub async fn mcp_management_catalog(
        &self,
        host: &str,
        token: &str,
    ) -> Result<McpCatalogResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone()).agent_apps_read_request(
            "/v1/agent-apps/mcp",
            &[("targetHostId", host.into())],
            Some(token),
        )
        .await
    }

    /// Read disabled built-in preset drafts without connecting to their servers.
    pub async fn mcp_management_presets(
        &self,
        host: &str,
        token: &str,
    ) -> Result<McpPresetsResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone()).agent_apps_read_request(
            "/v1/agent-apps/mcp/presets",
            &[("targetHostId", host.into())],
            Some(token),
        )
        .await
    }

    /// Read redacted native state for a trusted installation. Claude requires its managed account ID.
    pub async fn mcp_management_native(
        &self,
        target: &McpAppTarget,
        account_id: Option<&str>,
        token: &str,
    ) -> Result<McpNativePreview, AgentAppsReadError> {
        let serde_json::Value::String(app_id) = serde_json::to_value(target.app_id)
            .map_err(|err| AgentAppsReadError::Decode(err.to_string()))?
        else {
            return Err(AgentAppsReadError::Decode("invalid application identity".into()));
        };
        let mut query = vec![
            ("targetHostId", target.target_host_id.clone()),
            ("appId", app_id),
            ("installationId", target.installation_id.0.clone()),
        ];
        if let Some(account_id) = account_id {
            query.push(("accountId", account_id.into()));
        }
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone()).agent_apps_read_request("/v1/agent-apps/mcp/native", &query, Some(token)).await
    }
}

impl GatewayMcpReadClient {
    /// Copy all catalog definitions or one ID, skipping every protected definition.
    pub async fn mcp_management_export(
        &self, host: &str, id: Option<&str>, token: &str,
    ) -> Result<vw_api_mcp_management::McpSafeExportResponse, AgentAppsReadError> {
        let mut query = vec![("targetHostId", host.to_owned())];
        if let Some(id) = id { query.push(("id", id.to_owned())); }
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone()).agent_apps_read_request(
            "/v1/agent-apps/mcp/export", &query, Some(token),
        ).await
    }
}
