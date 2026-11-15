//! MCP 网关检查接口。
//!
//! 该接口按可选项目目录查询网关当前解析出的 MCP 服务状态与配置视图。

use super::GatewayClient;
use vw_api_mcp_inspect::McpInspectResponse;

impl GatewayClient {
    /// Call only through the host-issued binding for this exact session and server.
    pub async fn mcp_app_call(
        &self,
        session: &str,
        server: &str,
        binding: &str,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        gateway_mcp::GatewayMcpClient::new(self.transport.clone())
            .mcp_app_call(session, server, binding, name, arguments).await
    }

    /// Inspect configured MCP servers and their currently exposed tools.
    pub async fn mcp_inspect(&self) -> Result<McpInspectResponse, String> {
        gateway_mcp::GatewayMcpClient::new(self.transport.clone()).mcp_inspect().await
    }
}
