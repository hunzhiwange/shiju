//! Typed catalog mutations on the selected gateway; native writes stay on its execution host.
use super::{AgentAppsSettingsError, GatewayClient};
use vw_api_mcp_management::{
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
        gateway_mcp_write::GatewayMcpWriteClient::new(self.transport.clone())
            .mcp_management_create(request, token).await
    }

    /// Replace a connection at the expected revision, preserving display metadata.
    pub async fn mcp_management_update(
        &self,
        id: &McpRecordId,
        request: &McpUpdateRequest,
        token: &str,
    ) -> Result<McpCatalogRecord, AgentAppsSettingsError> {
        gateway_mcp_write::GatewayMcpWriteClient::new(self.transport.clone())
            .mcp_management_update(id, request, token).await
    }

    /// Retire the expected revision, preserving each native cleanup outcome.
    pub async fn mcp_management_delete(
        &self,
        id: &McpRecordId,
        request: &McpDeleteRequest,
        token: &str,
    ) -> Result<McpDistributionResult, AgentAppsSettingsError> {
        gateway_mcp_write::GatewayMcpWriteClient::new(self.transport.clone())
            .mcp_management_delete(id, request, token).await
    }
}
