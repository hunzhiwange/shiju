//! Typed reads from the selected gateway's managed MCP catalog.
use super::{AgentAppsReadError, GatewayClient};
use vw_api_mcp_management::{
    McpAppTarget, McpCatalogResponse, McpNativePreview, McpPresetsResponse,
};

impl GatewayClient {
    /// Read committed references for one explicitly authorized inbound ACP session.
    pub async fn mcp_management_acp_usage(
        &self,
        query: &vw_api_mcp_management::McpAcpUsageQuery,
        token: &str,
    ) -> Result<vw_api_mcp_management::McpAcpUsage, AgentAppsReadError> {
        gateway_mcp_read::GatewayMcpReadClient::new(self.transport.clone())
            .mcp_management_acp_usage(query, token).await
    }

    /// Read public catalog records, desired bindings and native adapter capabilities.
    pub async fn mcp_management_catalog(
        &self,
        host: &str,
        token: &str,
    ) -> Result<McpCatalogResponse, AgentAppsReadError> {
        gateway_mcp_read::GatewayMcpReadClient::new(self.transport.clone())
            .mcp_management_catalog(host, token).await
    }

    /// Read disabled built-in preset drafts without connecting to their servers.
    pub async fn mcp_management_presets(
        &self,
        host: &str,
        token: &str,
    ) -> Result<McpPresetsResponse, AgentAppsReadError> {
        gateway_mcp_read::GatewayMcpReadClient::new(self.transport.clone())
            .mcp_management_presets(host, token).await
    }

    /// Read redacted native state for a trusted installation. Claude requires its managed account ID.
    pub async fn mcp_management_native(
        &self,
        target: &McpAppTarget,
        account_id: Option<&str>,
        token: &str,
    ) -> Result<McpNativePreview, AgentAppsReadError> {
        gateway_mcp_read::GatewayMcpReadClient::new(self.transport.clone())
            .mcp_management_native(target, account_id, token).await
    }
}
