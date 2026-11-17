//! Compose MCP loading with the active tenant turn selection.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use agent_plugins::mcp::plugin_mcp_discovery_options;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_plugins::mcp::{
    plugin_mcp_discovery_options, load_plugin_mcp_declarations,
};
#[cfg(test)]
pub(crate) use agent_plugins::mcp::{load_plugin_mcp_declarations_from_path, load_portable_mcp_declarations_from_path};
use super::PluginDiscoveryOptions;
use agent_client_protocol::McpServer;
use anyhow::Result;
pub(crate) fn merge_enabled_plugin_mcp_servers(servers: &mut Vec<McpServer>, options: &PluginDiscoveryOptions) -> Result<()> {
    agent_plugins::mcp::merge_enabled_plugin_mcp_servers(servers, options, &super::compat::session_snapshot::selection_for_runtime)
}

#[cfg(test)]
pub(crate) fn load_plugin_mcp_servers_from_path(path: &std::path::Path) -> Result<Vec<McpServer>> {
    load_plugin_mcp_declarations_from_path(path)?.into_runtime_servers(&path.to_string_lossy())
}
