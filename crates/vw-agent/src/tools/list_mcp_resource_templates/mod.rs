//! 列出 MCP 服务器资源模板的工具实现。

use super::mcp_common::{load_resolved_acp_config, server_name, server_summary, with_server_peer};
use super::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use rmcp::service::Peer;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct Args {
    #[serde(default)]
    server: Option<String>,
}

/// 查询 MCP 资源模板列表的只读工具。
pub struct ListMcpResourceTemplatesTool;

impl ListMcpResourceTemplatesTool {
    /// 创建 MCP 资源模板列表工具。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ListMcpResourceTemplatesTool {
    fn name(&self) -> &str {
        "ListMcpResourceTemplates"
    }

    fn description(&self) -> &str {
        "列出当前会话可真实连通的 MCP 服务器资源模板。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "server": {
                    "type": "string",
                    "description": "可选的 MCP server 名称；未提供时遍历全部已配置 server。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("ListMcpResourceTemplates")
            .with_aliases(vec!["list_mcp_resource_templates".to_string()])
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        let config = load_resolved_acp_config().await?;
        let server_filter = args.server.as_deref().map(str::trim).filter(|value| !value.is_empty());
        let servers = config
            .mcp_servers
            .iter()
            .filter(|server| server_filter.is_none_or(|filter| server_name(server) == filter))
            .collect::<Vec<_>>();

        let mut items = Vec::new();
        for server in servers {
            let summary = server_summary(server);
            let resource_templates =
                with_server_peer(server, |peer: &Peer<rmcp::service::RoleClient>| {
                    Box::pin(async move {
                        let resource_templates = peer.list_all_resource_templates().await?;
                        Ok(resource_templates)
                    })
                })
                .await?;
            items.push(json!({
                "server": summary,
                "resource_templates": resource_templates,
            }));
        }

        Ok(ToolResult { success: true, output: serde_json::to_string_pretty(&items)?, error: None })
    }
}

#[cfg(test)]
mod tests;
