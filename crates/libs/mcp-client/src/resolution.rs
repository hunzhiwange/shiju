//! Resolve MCP configuration in precedence order using trusted host inputs.
use std::{borrow::Cow, collections::BTreeSet, future::Future, path::Path};
use agent_client_protocol::McpServer;
use acp_config::ResolvedAcpxConfig;
use serde_json::Value;
use crate::{apply_mcp_server_override_values, merge_global_mcp_server_values, server_name};

/// Borrowed fields from the host's validated agent configuration.
pub struct McpConfigValues<'a> {
    pub servers: &'a [Value],
    pub overrides: &'a [Value],
    pub source_path: Cow<'a, str>,
}

/// Existing configuration storage and plugin discovery actions.
pub trait McpConfigHost {
    type Config;
    fn load(&self, workspace: &Path) -> impl Future<Output = anyhow::Result<Self::Config>> + Send;
    fn values(config: &Self::Config) -> McpConfigValues<'_>;
    fn merge_plugins(&self, config: &mut Self::Config, workspace: &Path, servers: &mut Vec<McpServer>) -> anyhow::Result<()>;
}

/// Project-scoped managed server discovery and the current trusted tool selection.
pub trait RuntimeMcpConfigHost: McpConfigHost {
    fn managed_servers(&self, config: &Self::Config, workspace: &Path, servers: &[McpServer]) -> anyhow::Result<Vec<McpServer>>;
    fn native_selection(&self) -> Option<BTreeSet<String>>;
}

async fn load_with_plugins<H: McpConfigHost>(
    workspace: &Path,
    host: &H,
) -> anyhow::Result<(ResolvedAcpxConfig, H::Config)> {
    let mut resolved = acp_config::load_resolved_config(workspace).await.map_err(anyhow::Error::from)?;
    let mut config = host.load(workspace).await?;
    {
        let values = H::values(&config);
        merge_global_mcp_server_values(&mut resolved, values.servers, &values.source_path)?;
    }
    host.merge_plugins(&mut config, workspace, &mut resolved.mcp_servers)?;
    Ok((resolved, config))
}

/// ACP entries precede globals, then plugins; explicit management overrides apply last.
pub async fn resolve_mcp_config_with_plugins<H: McpConfigHost>(
    workspace: &Path,
    host: &H,
) -> anyhow::Result<ResolvedAcpxConfig> {
    let (mut resolved, config) = load_with_plugins(workspace, host).await?;
    let values = H::values(&config);
    apply_mcp_server_override_values(&mut resolved, values.overrides, &values.source_path)?;
    Ok(resolved)
}

/// Managed entries follow plugins; selection filters servers only after all overrides.
pub async fn resolve_runtime_mcp_config<H: RuntimeMcpConfigHost>(
    workspace: &Path,
    host: &H,
) -> anyhow::Result<ResolvedAcpxConfig> {
    let (mut resolved, config) = load_with_plugins(workspace, host).await?;
    let managed = host.managed_servers(&config, workspace, &resolved.mcp_servers)?;
    resolved.mcp_servers.extend(managed);
    let values = H::values(&config);
    apply_mcp_server_override_values(&mut resolved, values.overrides, &values.source_path)?;
    if let Some(selected) = host.native_selection() {
        resolved.mcp_servers.retain(|server| selected.contains(server_name(server)));
    }
    Ok(resolved)
}
