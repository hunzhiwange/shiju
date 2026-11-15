//! 已发现插件上下文的组合入口；解析及运行时重验由 plugin-mcp 持有。
use crate::plugins::{DiscoveredPlugin, PluginDiscoveryOptions};
use agent_client_protocol::McpServer;
use anyhow::Result;
use std::collections::HashMap;
use super::mcp_legacy::PluginMcpParseOutcome;
pub(crate) use plugin_mcp::mcp_environment::runtime_cwd;

pub(crate) fn resolve_servers(
    outcome: PluginMcpParseOutcome,
    plugin: &DiscoveredPlugin,
    options: &PluginDiscoveryOptions,
    source: &str,
    original_names: &HashMap<String, String>,
) -> Result<Vec<McpServer>> {
    plugin_mcp::mcp_environment::resolve_servers(
        outcome, &plugin.manifest, &plugin.source_dir, &plugin.identity,
        &options.config_dir, source, original_names,
    )
}
