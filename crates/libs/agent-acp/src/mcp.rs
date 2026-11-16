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
