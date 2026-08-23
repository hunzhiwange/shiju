//! MCP 网关检查接口。
//!
//! 该接口按可选项目目录查询网关当前解析出的 MCP 服务状态与配置视图。

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
