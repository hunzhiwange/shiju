//! MCP inspection composed with the scoped plugin host.
pub async fn inspect_mcp_servers(config: &acp_config::ResolvedAcpxConfig) -> vw_api_mcp_inspect::McpInspectResponse {
    tool_mcp::mcp_inspect::inspect_mcp_servers::<super::context::AgentContextHost>(config).await
}
pub async fn inspect_mcp_servers_from_workspace(workspace_dir: impl AsRef<std::path::Path>) -> anyhow::Result<vw_api_mcp_inspect::McpInspectResponse> {
    tool_mcp::mcp_inspect::inspect_mcp_servers_from_workspace::<super::context::AgentContextHost>(workspace_dir).await
}
#[cfg(test)]
use vw_api_mcp_inspect::McpConnectionStatusDto;
#[cfg(test)]
#[path = "../../../libs/tool-mcp/src/mcp_inspect_tests.rs"]
mod mcp_inspect_tests;
