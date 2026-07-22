use super::GatewayClient;
use vw_api_types::mcp::McpInspectResponse;

impl GatewayClient {
    /// Inspect configured MCP servers and their currently exposed tools.
    pub async fn mcp_inspect(&self) -> Result<McpInspectResponse, String> {
        self.get_json("/v1/mcp/inspect", &[]).await
    }
}

#[cfg(test)]
#[path = "mcp_api_tests.rs"]
mod mcp_api_tests;
