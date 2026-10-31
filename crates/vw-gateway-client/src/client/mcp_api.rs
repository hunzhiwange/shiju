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

impl GatewayClient {
    pub async fn mcp_form_open(
        &self,
        session_id: &str,
        workspace: Option<String>,
    ) -> Result<vw_api_types::mcp_form::McpFormHostOpened, String> {
        self.post_json(
            "/v1/mcp/forms/open",
            &[],
            &vw_api_types::mcp_form::McpFormHostOpen {
                session_id: session_id.to_owned(),
                workspace,
            },
        )
        .await
    }
    pub async fn mcp_form_poll(
        &self,
        host_id: &str,
    ) -> Result<Vec<vw_api_types::mcp_form::McpFormRequest>, String> {
        self.post_json(
            "/v1/mcp/forms/poll",
            &[],
            &vw_api_types::mcp_form::McpFormHostRef { host_id: host_id.to_owned() },
        )
        .await
    }
    pub async fn mcp_form_reply(
        &self,
        body: &vw_api_types::mcp_form::McpFormHostReply,
    ) -> Result<bool, String> {
        self.post_json("/v1/mcp/forms/reply", &[], body).await
    }
    pub async fn mcp_form_close(&self, host_id: &str) -> Result<bool, String> {
        self.post_json(
            "/v1/mcp/forms/close",
            &[],
            &vw_api_types::mcp_form::McpFormHostRef { host_id: host_id.to_owned() },
        )
        .await
    }
}

#[cfg(test)]
#[path = "mcp_api_tests.rs"]
mod mcp_api_tests;
