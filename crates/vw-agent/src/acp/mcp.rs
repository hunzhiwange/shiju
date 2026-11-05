//! ACP 会话级 MCP 配置的验证、启动探测与动态工具发现。

use std::collections::HashSet;
use std::sync::Arc;

use agent_client_protocol::McpServer;

use crate::app::agent::tools::Tool;
use crate::session::runtime::SessionToolset;
use crate::tools::mcp_common::server_name;
use vw_acp::server::BackendError;

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

fn validate_server_names(servers: &[McpServer]) -> Result<(), BackendError> {
    let mut names = HashSet::new();
    for server in servers {
        let name = supported_server_name(server)?;
        if !names.insert(name.to_string()) {
            return Err(BackendError::InvalidRequest(format!("duplicate MCP server name: {name}")));
        }
    }
    Ok(())
}

/// 完整探测所有配置；任何一项失败都不返回部分工具。
pub(crate) async fn prepare_session_tools(
    servers: &[McpServer],
) -> Result<SessionToolset, BackendError> {
    validate_server_names(servers)?;
    let connections = Arc::new(crate::plugins::McpSessionConnections::default());
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
                vw_acp::McpServerManagement::default_for_server(server_name(server)),
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

    Ok(lease.with_catalog(Arc::new(
        crate::tools::catalog::ToolCatalogSnapshot::from_runtime_tools(tools),
    )))
}
