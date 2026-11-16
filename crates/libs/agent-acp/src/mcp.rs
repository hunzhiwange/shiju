//! 入站会话 MCP 配置的传输与名称验证。
use std::collections::HashSet;
use agent_client_protocol::McpServer;
use acp_server::server::BackendError;

fn supported_server_name(server: &McpServer) -> Result<&str, BackendError> {
    let name = match server {
        McpServer::Http(server) => &server.name,
        McpServer::Sse(server) => &server.name,
        McpServer::Stdio(server) => &server.name,
        _ => {
            return Err(BackendError::InvalidRequest(
                "unsupported MCP server transport".to_string(),
            ));
        }
    };
    if name.trim().is_empty() {
        return Err(BackendError::InvalidRequest("MCP server name must not be empty".to_string()));
    }
    Ok(name)
}

pub fn validate_server_names(servers: &[McpServer]) -> Result<(), BackendError> {
    let mut names = HashSet::new();
    for server in servers {
        let name = supported_server_name(server)?;
        if !names.insert(name.to_string()) {
            return Err(BackendError::InvalidRequest(format!("duplicate MCP server name: {name}")));
        }
    }
    Ok(())
}

use std::sync::Arc;
use tool_types::Tool;
use tool_assembly::catalog::ToolCatalogSnapshot;
use plugin_mcp::mcp_connections::McpSessionConnections;
use mcp_client::server_name;
pub type SessionToolset = session_port::SessionToolset<ToolCatalogSnapshot, McpSessionConnections>;

/// 完整探测所有配置；任何一项失败都不返回部分工具。
pub async fn prepare_session_tools(
    servers: &[McpServer],
    connections: Arc<McpSessionConnections>,
) -> Result<SessionToolset, BackendError> {
    validate_server_names(servers)?;
    let lease = SessionToolset::default().with_mcp_connections(connections.clone());
    connections
        .synchronize(servers)
        .map_err(|error| BackendError::InvalidRequest(error.to_string()))?;
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    let mut existing_ids = HashSet::new();

    for server in servers {
        let reserved = existing_ids.iter().map(String::as_str).collect::<Vec<_>>();
        let discovered = connections
            .discover_catalog_tools(
                server,
                acp_config::McpServerManagement::default_for_server(server_name(server)),
                &reserved,
            )
            .await
            .map_err(|_| {
                BackendError::InvalidRequest(format!(
                    "MCP server '{}' failed to start",
                    server_name(server)
                ))
            })?;

        for tool in discovered {
            existing_ids.insert(tool.name().to_string());
            tools.push(tool);
        }
    }

    Ok(lease.with_catalog(Arc::new(ToolCatalogSnapshot::from_runtime_tools(tools))))
}
