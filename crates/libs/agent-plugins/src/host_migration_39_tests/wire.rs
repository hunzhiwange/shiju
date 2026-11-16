use agent_client_protocol::McpServer;
use plugin_mcp::mcp_connections::McpSessionConnections;
use std::path::Path;
pub(super) fn merge_enabled_plugin_mcp_servers(servers: &mut Vec<McpServer>, options: &crate::PluginDiscoveryOptions) -> anyhow::Result<()> {
    crate::mcp::merge_enabled_plugin_mcp_servers(servers, options, &|options| Ok((crate::discover_plugins(options)?, options.clone())))
}
pub(super) fn remove_installed_plugin(root: &Path, name: &str) -> anyhow::Result<()> {
    crate::install::remove_installed_plugin(root, name, &plugin_mcp::mcp_connection_sessions::close_plugin)
}
pub(super) fn set_plugin_enabled(root: &Path, name: &str, enabled: bool) -> anyhow::Result<()> {
    crate::install::set_plugin_enabled(root, name, enabled, &plugin_mcp::mcp_connection_sessions::close_plugin)
}
pub(super) async fn call_wire(pool: &McpSessionConnections, server: &McpServer, name: &str, tool: &'static str, args: serde_json::Value) -> anyhow::Result<tool_types::ToolCallResult> {
    let result = pool.with_peer(server, None, |peer| Box::pin(async move {
        Ok(peer.call_tool(rmcp::model::CallToolRequestParam { name: tool.into(), arguments: args.as_object().cloned() }).await?)
    })).await?;
    plugin_mcp::mcp_content::normalize_mcp_tool_result(name, tool, result)
}
