//! MCP 认证桥接信息查询工具。
//!
//! 工具只读取当前解析后的 ACP 配置，返回认证策略、已配置认证方法名称和目标
//! server 摘要；不会输出密钥、令牌或任何原始敏感载荷。

use super::mcp_common::{auth_method_names, load_resolved_acp_config, server_summary};
use super::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct Args {
    #[serde(default)]
    server: Option<String>,
}

/// 查看 MCP 认证配置摘要的只读工具。
///
/// 可选 `server` 参数用于只返回某个 server 的桥接信息。配置读取或 JSON 序列化失败
/// 会通过 `anyhow::Result` 返回。
pub struct McpAuthTool;

impl McpAuthTool {
    /// 创建 MCP 认证信息工具。
    ///
    /// 返回无状态工具实例；具体配置在执行阶段读取。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for McpAuthTool {
    fn name(&self) -> &str {
        "McpAuth"
    }

    fn description(&self) -> &str {
        "读取当前会话的 MCP 认证桥接信息，包括 authPolicy、已配置认证方法和目标 server。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "server": {
                    "type": "string",
                    "description": "可选的 server 名称，用于只返回目标 server 的桥接信息。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("McpAuth")
            .with_aliases(vec!["mcp_auth".to_string()])
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(true)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if !args.is_object() {
            anyhow::bail!("invalid type: expected an object");
        }
        let args: Args = serde_json::from_value(args)?;
        let config = load_resolved_acp_config().await?;
        let server_filter = args.server.as_deref().map(str::trim).filter(|value| !value.is_empty());
        let servers = config
            .mcp_servers
            .iter()
            .filter(|server| {
                server_filter.is_none_or(|filter| match server {
                    agent_client_protocol::McpServer::Http(server) => server.name == filter,
                    agent_client_protocol::McpServer::Sse(server) => server.name == filter,
                    agent_client_protocol::McpServer::Stdio(server) => server.name == filter,
                    _ => false,
                })
            })
            .map(server_summary)
            .collect::<Vec<_>>();

        // 这里只暴露认证方法名称，不序列化认证体，避免日志或工具输出泄露凭据。
        let data = json!({
            "auth_policy": config.auth_policy,
            "auth_methods": auth_method_names(&config.auth),
            "servers": servers,
        });
        Ok(ToolResult { success: true, output: serde_json::to_string_pretty(&data)?, error: None })
    }
}
#[cfg(test)]
mod tests;
