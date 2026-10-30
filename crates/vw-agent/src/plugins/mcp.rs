//! 从已启用插件中加载 MCP 服务器配置。
//!
//! 加载结果需经过清单路径审计，并与用户配置按稳定优先级合并。

use crate::app::agent::config::Config;
use agent_client_protocol::McpServer;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::PluginManifest;
use super::compat::mcp_legacy::{PluginMcpParseOutcome, parse_legacy_mcp};
use super::compat::mcp_portable::parse_portable_mcp;
use super::{PluginDiscoveryOptions, discover_plugins};

/// Build plugin discovery inputs from the loaded agent runtime config.
/// 根据代理配置构建插件 MCP 发现范围。
pub(crate) fn plugin_mcp_discovery_options(config: &Config) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        plugin_config_dir(config),
        config.plugins.clone(),
    )
}

/// Read a plugin `.mcp.json` file and parse its MCP server declarations.
/// 读取并严格解析单个插件声明的 MCP 服务器列表。
#[cfg(test)]
pub(crate) fn load_plugin_mcp_servers_from_path(path: &Path) -> Result<Vec<McpServer>> {
    load_plugin_mcp_declarations_from_path(path)?.into_runtime_servers(&path.to_string_lossy())
}

/// 单项诊断保留合法兄弟声明；实际运行准入仍拒绝含错误的整份配置。
pub(crate) fn load_plugin_mcp_declarations_from_path(path: &Path) -> Result<PluginMcpParseOutcome> {
    parse_legacy_mcp(read_mcp_document(path)?, &path.to_string_lossy())
}

pub(crate) fn load_portable_mcp_declarations_from_path(
    path: &Path,
) -> Result<PluginMcpParseOutcome> {
    parse_portable_mcp(read_mcp_document(path)?)
}

fn read_mcp_document(path: &Path) -> Result<Value> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read plugin MCP config: {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse plugin MCP config: {}", path.display()))
}

/// Append MCP servers from enabled plugins after the user's ACP servers.
/// 将已启用插件的 MCP 服务器追加到用户配置。
///
/// 用户已声明的同名服务器保持优先，插件之间的重名项按发现顺序只接受一次。
pub(crate) fn merge_enabled_plugin_mcp_servers(
    servers: &mut Vec<McpServer>,
    options: &PluginDiscoveryOptions,
) -> Result<()> {
    let discovery = discover_plugins(options).context("failed to discover plugin MCP servers")?;
    let user_server_names =
        servers.iter().map(|server| mcp_server_name(server).to_string()).collect::<HashSet<_>>();
    let mut merged_names = user_server_names.clone();

    for plugin in discovery.enabled {
        let (outcome, source_path) = load_plugin_mcp_declarations(&plugin.manifest)?;
        let plugin_servers = super::compat::mcp_environment::resolve_servers(
            outcome,
            &plugin,
            options,
            &source_path.to_string_lossy(),
        )
        .with_context(|| {
            format!(
                "failed to load MCP servers for plugin '{}': {}",
                plugin.manifest.name,
                source_path.display()
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

/// 发现与运行准入共用声明入口；便携包缺省 mcp.json 可不存在，但已有路径须重新审计。
pub(crate) fn load_plugin_mcp_declarations(
    manifest: &PluginManifest,
) -> Result<(PluginMcpParseOutcome, PathBuf)> {
    let portable = manifest.manifest_path == manifest.root.join("plugin.json");
    let source = match &manifest.mcp_servers {
        Some(source) => source.clone(),
        None => {
            let legacy = manifest
                .manifest_path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    matches!(name, ".codex-plugin" | ".claude-plugin" | ".cursor-plugin")
                });
            if !legacy {
                return Ok((PluginMcpParseOutcome::default(), manifest.manifest_path.clone()));
            }
            if fs::symlink_metadata(manifest.root.join(".mcp.json"))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                return Ok((PluginMcpParseOutcome::default(), manifest.root.join(".mcp.json")));
            }
            super::PluginMcpServers::Path(super::audit::resolve_component_path(
                &manifest.root,
                "mcpServers",
                "./.mcp.json",
            )?)
        }
    };
    let source_path = match &source {
        super::PluginMcpServers::Path(path) => path.canonical_path.clone(),
        super::PluginMcpServers::Inline(_) => manifest.manifest_path.clone(),
    };
    let result: Result<PluginMcpParseOutcome> = (|| {
        let mut outcome = match source {
            super::PluginMcpServers::Path(path) if portable => {
                if fs::symlink_metadata(manifest.root.join("mcp.json"))
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
                {
                    return Ok(PluginMcpParseOutcome::default());
                }
                let path =
                    super::audit::resolve_component_path(&manifest.root, "mcpServers", &path.raw)?;
                load_portable_mcp_declarations_from_path(&path.canonical_path)?
            }
            super::PluginMcpServers::Path(path) => {
                load_plugin_mcp_declarations_from_path(&path.canonical_path)?
            }
            super::PluginMcpServers::Inline(servers) => {
                parse_legacy_mcp(Value::Object(servers), &source_path.to_string_lossy())?
            }
        };
        if portable {
            super::compat::mcp_portable_overlay::apply_env_overlay(&manifest.root, &mut outcome)?;
        }
        Ok(outcome)
    })();
    result
        .with_context(|| {
            format!(
                "failed to load MCP servers for plugin '{}': {}",
                manifest.name,
                source_path.display()
            )
        })
        .map(|outcome| (outcome, source_path))
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
