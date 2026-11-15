//! MCP tool composition using the application's scoped session services.
pub type McpTool = tool_mcp::mcp_tool::McpTool<super::context::AgentContextHost>;
pub use tool_mcp::mcp_tool::{mcp_tool_id, mcp_tools_to_specs, discover_mcp_tool_specs};

impl tool_mcp::McpToolHost for super::context::AgentContextHost {
    type Connections = crate::plugins::McpSessionConnections;
    fn new_connections() -> Self::Connections {
        crate::plugins::McpSessionConnections::new(crate::plugins::compat::mcp_connections::host())
    }
    async fn load_config(workspace: Option<&std::path::Path>) -> anyhow::Result<acp_config::ResolvedAcpxConfig> {
        match workspace {
            Some(path) => super::mcp_common::load_resolved_acp_config_from(path).await,
            None => super::mcp_common::load_resolved_acp_config().await,
        }
    }
    fn connection_required(server: &agent_client_protocol::McpServer) -> anyhow::Result<bool> {
        Ok(crate::plugins::plugin_mcp_connection_settings(server)?.required)
    }
    fn missing_executable(server: &agent_client_protocol::McpServer) -> anyhow::Result<Option<(String, String)>> {
        Ok(crate::plugins::plugin_missing_mcp_executable(server)?.map(|item| (item.dependency, item.detail)))
    }
    fn tool_allowed(server: &agent_client_protocol::McpServer, name: &str) -> anyhow::Result<bool> {
        crate::plugins::plugin_mcp_tool_allowed(server, name)
    }
    fn tool_binding(connections: &Self::Connections, server: &agent_client_protocol::McpServer) -> Option<mcp_client::ToolCatalogBinding> {
        connections.tool_binding(server)
    }
    async fn with_client<T: Send, F>(server: &agent_client_protocol::McpServer, timeout: Option<std::time::Duration>, f: F) -> anyhow::Result<T>
    where F: for<'a> FnOnce(&'a mcp_client::RunningMcpPeer) -> tool_mcp::ClientFuture<'a, T> + Send {
        super::mcp_common::with_server_client_timeout(server, timeout, f).await
    }
    async fn with_connection<T: Send, F>(connections: &Self::Connections, server: &agent_client_protocol::McpServer, timeout: Option<std::time::Duration>, f: F) -> anyhow::Result<T>
    where F: for<'a> FnOnce(&'a mcp_client::RunningMcpPeer) -> tool_mcp::ClientFuture<'a, T> + Send {
        connections.with_client(server, timeout, f).await
    }
    fn app_resource<'a>(client: &'a mcp_client::RunningMcpPeer, binding: &'a mut serde_json::Value) -> tool_mcp::ClientFuture<'a, ()> {
        Box::pin(crate::plugins::mcp_apps::resource(client, binding))
    }
    fn call_resource<'a>(config: &'a acp_config::ResolvedAcpxConfig, connections: &'a std::sync::Arc<Self::Connections>, input: serde_json::Value) -> tool_mcp::ClientFuture<'a, tool_types::ToolCallResult> {
        Box::pin(crate::plugins::call_mcp_resource(config, connections, input))
    }
}
