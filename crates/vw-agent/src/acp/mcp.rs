//! ACP 会话级 MCP 配置的验证、启动探测与动态工具发现。

use std::collections::HashSet;
use std::sync::Arc;

use agent_client_protocol::McpServer;

use crate::app::agent::tools::Tool;
use crate::session::runtime::SessionToolset;
use crate::tools::mcp_common::{server_name, with_server_peer};
use crate::tools::mcp_tool::{McpTool, mcp_tools_to_specs};
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
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    let mut existing_ids = HashSet::new();

    for server in servers {
        let safe_name = supported_server_name(server)?.to_string();
        let configured_server = server.clone();
        let reserved = existing_ids.iter().cloned().collect::<Vec<String>>();
        let discovered = with_server_peer(server, move |peer| {
            let configured_server = configured_server.clone();
            let safe_name = safe_name.clone();
            let reserved = reserved.clone();
            Box::pin(async move {
                let remote_tools = peer.list_all_tools().await?;
                let reserved_refs = reserved.iter().map(String::as_str).collect::<Vec<_>>();
                let instructions = peer.peer_info().and_then(|info| info.instructions.as_deref());
                let specs =
                    mcp_tools_to_specs(&safe_name, instructions, &remote_tools, &reserved_refs)?;
                Ok(remote_tools
                    .into_iter()
                    .zip(specs)
                    .map(|(tool, spec)| {
                        McpTool::from_configured_server(
                            configured_server.clone(),
                            tool.name.to_string(),
                            spec,
                        )
                    })
                    .collect::<Vec<_>>())
            })
        })
        .await
        .map_err(|_| {
            BackendError::InvalidRequest(format!(
                "MCP server '{}' failed to start",
                server_name(server)
            ))
        })?;

        for tool in discovered {
            existing_ids.insert(tool.name().to_string());
            tools.push(Arc::new(tool));
        }
    }

    Ok(SessionToolset::new(tools))
}
