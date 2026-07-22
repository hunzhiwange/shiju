//! MCP server 状态诊断工具。

use super::mcp_common::{
    auth_method_names, load_resolved_acp_config, management_for_server, management_timeout,
    server_name, server_transport, unsupported_mcp_capabilities_json, with_server_peer_timeout,
};
use super::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Args {
    #[serde(default)]
    server: Option<String>,
}

pub struct McpStatusTool {
    workspace_dir: Option<PathBuf>,
}

impl McpStatusTool {
    pub fn new() -> Self {
        Self { workspace_dir: None }
    }

    #[cfg(test)]
    pub(crate) fn new_for_workspace(workspace_dir: impl Into<PathBuf>) -> Self {
        Self { workspace_dir: Some(workspace_dir.into()) }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for McpStatusTool {
    fn name(&self) -> &str {
        "McpStatus"
    }

    fn description(&self) -> &str {
        "读取当前 MCP server 的启用状态、连接状态、工具数量和认证方法摘要。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "server": {
                    "type": "string",
                    "description": "可选的 server 名称，用于只返回目标 server 的状态。"
                }
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("McpStatus")
            .with_aliases(vec!["mcp_status".to_string()])
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        let config = match &self.workspace_dir {
            Some(workspace_dir) => {
                super::mcp_common::load_resolved_acp_config_from(workspace_dir).await?
            }
            None => load_resolved_acp_config().await?,
        };
        let server_filter = args.server.as_deref().map(str::trim).filter(|value| !value.is_empty());
        let auth_methods = auth_method_names(&config.auth);
        let mut servers = Vec::new();

        for server in &config.mcp_servers {
            let name = server_name(server);
            if server_filter.is_some_and(|filter| filter != name) {
                continue;
            }

            let management = management_for_server(&config, server);
            let mut entry = json!({
                "name": name,
                "transport": server_transport(server),
                "enabled": management.enabled,
                "required": management.required,
                "connection_status": "disabled",
                "tool_count": Value::Null,
                "auth_methods": auth_methods.clone(),
                "allowed_tools": management.allowed_tools.clone(),
                "denied_tools": management.denied_tools.clone(),
            });

            if let Some(timeout_ms) = management.timeout_ms {
                entry["timeout_ms"] = json!(timeout_ms);
            }

            if management.enabled {
                match inspect_server(server, &management).await {
                    Ok(tool_count) => {
                        entry["connection_status"] = json!("connected");
                        entry["tool_count"] = json!(tool_count);
                    }
                    Err(error) => {
                        entry["connection_status"] = json!("error");
                        entry["error"] = json!(summarize_error(&error));
                    }
                }
            }

            servers.push(entry);
        }

        let data = json!({
            "auth_policy": config.auth_policy,
            "auth_methods": auth_methods,
            "unsupported_capabilities": unsupported_mcp_capabilities_json(),
            "servers": servers,
        });
        Ok(ToolResult { success: true, output: serde_json::to_string_pretty(&data)?, error: None })
    }
}

async fn inspect_server(
    server: &agent_client_protocol::McpServer,
    management: &vw_acp::McpServerManagement,
) -> anyhow::Result<usize> {
    with_server_peer_timeout(server, management_timeout(management), |peer| {
        Box::pin(async move {
            let tools = peer.list_all_tools().await?;
            Ok(tools.len())
        })
    })
    .await
}

fn summarize_error(error: &anyhow::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests;
