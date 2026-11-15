//! MCP 分发兼容门面。
pub use gateway_mcp_distribution::*;
use super::GatewayClient;
use vw_api_transport::TaskId;
use vw_api_app_operations::AppActionExecutionResult;
use vw_api_mcp_management::{McpRecordId, McpDistributionResult, McpResyncOperationStatus};

impl GatewayClient {
    pub async fn mcp_management_import(
        &self,
        request: &McpImportRequest,
        token: &str,
    ) -> Result<McpImportResponse, AgentAppsSettingsError> {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(self.transport.clone())
            .mcp_management_import(request, token).await
    }

    pub async fn mcp_management_bind(
        &self,
        id: &McpRecordId,
        request: &McpBindingRequest,
        token: &str,
    ) -> Result<McpDistributionResult, AgentAppsSettingsError> {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(self.transport.clone())
            .mcp_management_bind(id, request, token).await
    }

    pub async fn mcp_management_bulk(
        &self,
        request: &McpBulkRequest,
        token: &str,
    ) -> Result<McpDistributionResult, AgentAppsSettingsError> {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(self.transport.clone())
            .mcp_management_bulk(request, token).await
    }

    pub async fn mcp_management_resync(
        &self,
        request: &McpResyncRequest,
        token: &str,
    ) -> Result<AppActionExecutionResult, AgentAppsSettingsError> {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(self.transport.clone())
            .mcp_management_resync(request, token).await
    }

    pub async fn mcp_management_resync_status(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<McpResyncOperationStatus, AgentAppsLifecycleError> {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(self.transport.clone())
            .mcp_management_resync_status(host, id, token).await
    }

    pub async fn mcp_management_resync_cancel(
        &self,
        host: &str,
        id: &TaskId,
        token: &str,
    ) -> Result<McpResyncOperationStatus, AgentAppsLifecycleError> {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(self.transport.clone())
            .mcp_management_resync_cancel(host, id, token).await
    }
}
