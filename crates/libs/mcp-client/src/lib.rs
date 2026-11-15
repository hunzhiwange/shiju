//! MCP server metadata, management policy and configuration merging.

use agent_client_protocol::McpServer;
use rmcp::model::ResourceContents;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};

mod config;

pub mod protocol;
pub mod notifications;
pub mod catalog;
pub mod capabilities;
pub mod wire_result;

pub use protocol::{FormOwner, RawResponse as RawMcpResponse};
pub use notifications::{McpCatalogs, ToolCatalogBinding};
pub use config::{apply_mcp_server_override_values, merge_global_mcp_server_values};

pub fn management_for_server(
    config: &acp_config::ResolvedAcpxConfig,
    server: &McpServer,
) -> acp_config::McpServerManagement {
    let name = server_name(server);
    config
        .mcp_server_management
        .iter()
        .find(|management| management.name == name)
        .cloned()
        .unwrap_or_else(|| acp_config::McpServerManagement::default_for_server(name))
}

pub fn management_timeout(management: &acp_config::McpServerManagement) -> Option<Duration> {
    management.timeout_ms.map(Duration::from_millis)
}

pub fn mcp_tool_allowed(
    management: &acp_config::McpServerManagement,
    tool_name: &str,
    tool_id: &str,
) -> bool {
    if management.denied_tools.iter().any(|entry| tool_policy_matches(entry, tool_name, tool_id)) {
        return false;
    }
    management.allowed_tools.is_empty()
        || management
            .allowed_tools
            .iter()
            .any(|entry| tool_policy_matches(entry, tool_name, tool_id))
}

pub fn auth_method_names(auth: &HashMap<String, String>) -> Vec<String> {
    let mut methods = auth.keys().cloned().collect::<Vec<_>>();
    methods.sort_unstable();
    methods
}

fn tool_policy_matches(entry: &str, tool_name: &str, tool_id: &str) -> bool {
    entry == tool_name || entry == tool_id
}

/// 返回 MCP server 的稳定名称。
///
/// # 参数
///
/// - `server`: MCP server 配置枚举。
///
/// # 返回值
///
/// 对受支持传输返回配置中的名称；未知传输返回 `"unknown"`，用于诊断输出。
pub fn server_name(server: &McpServer) -> &str {
    match server {
        McpServer::Http(server) => &server.name,
        McpServer::Sse(server) => &server.name,
        McpServer::Stdio(server) => &server.name,
        _ => "unknown",
    }
}

pub fn server_transport(server: &McpServer) -> &'static str {
    match server {
        McpServer::Http(_) => "http",
        McpServer::Sse(_) => "sse",
        McpServer::Stdio(_) => "stdio",
        _ => "unknown",
    }
}

/// 按稳定名称查找 MCP server。
///
/// # 参数
///
/// - `servers`: 已解析配置中的 MCP server 列表。
/// - `name`: 用户提供的 server 名称，首尾空白会被忽略。
///
/// # 返回值
///
/// 找到时返回匹配的 server 引用。
///
/// # 错误
///
/// 当名称为空或配置中不存在对应 server 时返回明确错误。
pub fn find_server_by_name<'a>(
    servers: &'a [McpServer],
    name: &str,
) -> anyhow::Result<&'a McpServer> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("MCP server name must not be empty");
    }
    servers
        .iter()
        .find(|server| server_name(server) == name)
        .ok_or_else(|| anyhow::anyhow!("unknown MCP server '{}'", name))
}

/// 将 MCP server 的关键连接信息转换为可展示的 JSON 摘要。
///
/// # 参数
///
/// - `server`: MCP server 配置枚举。
///
/// # 返回值
///
/// 返回包含名称、传输类型及非敏感连接字段的 JSON 值。远程 headers 与 stdio 环境变量
/// 不会写入摘要，避免把潜在敏感配置暴露给工具输出。
pub fn server_summary(server: &McpServer) -> Value {
    match server {
        McpServer::Http(server) => json!({
            "name": server.name,
            "transport": "http",
            "url": server.url,
        }),
        McpServer::Sse(server) => json!({
            "name": server.name,
            "transport": "sse",
            "url": server.url,
        }),
        McpServer::Stdio(server) => json!({
            "name": server.name,
            "transport": "stdio",
            "command": server.command,
            "args": server.args,
        }),
        _ => json!({
            "name": "unknown",
            "transport": "unknown",
        }),
    }
}

/// 将 MCP 资源内容转换为工具输出使用的 JSON 数组。
///
/// # 参数
///
/// - `contents`: MCP `read_resource` 返回的资源内容列表。
///
/// # 返回值
///
/// 每个资源会保留 URI、MIME 类型、内容载荷以及 `kind` 字段，方便调用端区分文本与
/// 二进制资源。
pub fn resource_contents_json(contents: &[ResourceContents]) -> Value {
    Value::Array(
        contents
            .iter()
            .map(|content| match content {
                ResourceContents::TextResourceContents { uri, mime_type, text, .. } => {
                    json!({
                        "uri": uri,
                        "mime_type": mime_type,
                        "text": text,
                        "kind": "text",
                    })
                }
                ResourceContents::BlobResourceContents { uri, mime_type, blob, .. } => {
                    json!({
                        "uri": uri,
                        "mime_type": mime_type,
                        "blob": blob,
                        "kind": "blob",
                    })
                }
            })
            .collect(),
    )
}
