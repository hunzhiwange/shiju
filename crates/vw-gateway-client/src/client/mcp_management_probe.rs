//! Explicit MCP checks and status snapshots share the execution host's diagnostic jobs.
use super::{AgentAppsSettingsError, GatewayClient};
use {
    vw_api_transport::TaskId,
    vw_api_app_diagnostics::DiagnosticJobIdentity,
    vw_api_app_diagnostics::DiagnosticJobSnapshot,
    vw_api_app_diagnostics::DiagnosticScopeQuery,
    vw_api_mcp_management::McpRecordId,
};

impl GatewayClient {
    /// Discover exact catalog tools on the authorized host; acceptance does not grant invocation.
    pub async fn mcp_management_tools(
        &self,
        scope: &DiagnosticScopeQuery,
        request: &vw_api_mcp_management::McpToolDetailsRequest,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AgentAppsSettingsError> {
        gateway_mcp_probe::GatewayMcpProbeClient::new(self.transport.clone())
            .mcp_management_tools(scope, request, token).await
    }

    /// Start an explicit connection check for the caller generation.
    /// Acceptance is not connection success; match results by task, generation and revision.
    pub async fn mcp_management_check(
        &self,
        scope: &DiagnosticScopeQuery,
        server_ids: &[McpRecordId],
        generation: u64,
        token: &str,
    ) -> Result<DiagnosticJobIdentity, AgentAppsSettingsError> {
        gateway_mcp_probe::GatewayMcpProbeClient::new(self.transport.clone())
            .mcp_management_check(scope, server_ids, generation, token).await
    }

    /// Poll the original job's state, progress and revision-bound partial/final results.
    pub async fn mcp_management_check_status(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        gateway_mcp_probe::GatewayMcpProbeClient::new(self.transport.clone())
            .mcp_management_check_status(scope, task_id, token).await
    }

    /// Request cancellation. `Cancelling` is not confirmation that cleanup has finished.
    pub async fn mcp_management_check_cancel(
        &self,
        scope: &DiagnosticScopeQuery,
        task_id: &TaskId,
        token: &str,
    ) -> Result<DiagnosticJobSnapshot, AgentAppsSettingsError> {
        gateway_mcp_probe::GatewayMcpProbeClient::new(self.transport.clone())
            .mcp_management_check_cancel(scope, task_id, token).await
    }
}
