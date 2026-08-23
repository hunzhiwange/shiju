//! 从已启用插件中加载 MCP 服务器配置。
//!
//! 加载结果需经过清单路径审计，并与用户配置按稳定优先级合并。

use crate::app::agent::config::Config;
use agent_client_protocol::McpServer;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{PluginDiscoveryOptions, discover_plugins};

/// Build plugin discovery inputs from the loaded agent runtime config.
pub(crate) fn plugin_mcp_discovery_options(config: &Config) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        plugin_config_dir(config),
        config.plugins.clone(),
    )
}

/// Read a plugin `.mcp.json` file and parse its MCP server declarations.
pub(crate) fn load_plugin_mcp_servers_from_path(path: &Path) -> Result<Vec<McpServer>> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read plugin MCP config: {}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse plugin MCP config: {}", path.display()))?;
    let source_path = path.to_string_lossy();
    let servers = match &value {
        Value::Array(_) => &value,
        Value::Object(object) => object
            .get("mcpServers")
            .with_context(|| format!("plugin MCP config missing mcpServers: {}", path.display()))?,
        _ => bail!("Invalid plugin MCP config in {}: expected array or object", path.display()),
    };

    vw_acp::parse_mcp_servers(servers, &source_path).map_err(anyhow::Error::from)
}

/// Append MCP servers from enabled plugins after the user's ACP servers.
pub(crate) fn merge_enabled_plugin_mcp_servers(
    servers: &mut Vec<McpServer>,
    options: &PluginDiscoveryOptions,
) -> Result<()> {
    let discovery = discover_plugins(options).context("failed to discover plugin MCP servers")?;
    let user_server_names =
        servers.iter().map(|server| mcp_server_name(server).to_string()).collect::<HashSet<_>>();
    let mut merged_names = user_server_names.clone();

    for plugin in discovery.enabled {
        let Some(mcp_path) = plugin.manifest.mcp_servers else {
            continue;
        };
        let plugin_servers = load_plugin_mcp_servers_from_path(&mcp_path.canonical_path)
            .with_context(|| {
                format!(
                    "failed to load MCP servers for plugin '{}': {}",
                    plugin.manifest.name,
                    mcp_path.canonical_path.display()
                )
            })?;

        for server in plugin_servers {
            let name = mcp_server_name(&server);
            if user_server_names.contains(name) {
                tracing::warn!(
                    plugin = plugin.manifest.name.as_str(),
                    server = name,
                    "skipping plugin MCP server because ACP config already defines it"
                );
                continue;
            }
            if !merged_names.insert(name.to_string()) {
                tracing::warn!(
                    plugin = plugin.manifest.name.as_str(),
                    server = name,
                    "skipping duplicate plugin MCP server"
                );
                continue;
            }
            servers.push(server);
        }
    }

    Ok(())
}

fn plugin_config_dir(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
}

fn mcp_server_name(server: &McpServer) -> &str {
    match server {
        McpServer::Http(server) => &server.name,
        McpServer::Sse(server) => &server.name,
        McpServer::Stdio(server) => &server.name,
        _ => "unknown",
    }
}
