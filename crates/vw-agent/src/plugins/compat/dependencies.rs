//! Compose audited manifest/MCP loading and the existing skill inventory.
use agent_client_protocol::McpServer;
use anyhow::Result;
use std::path::Path;
pub use plugin_install::dependencies::{DependencyState, PluginDependencyCheck};

pub fn inspect_plugin_dependencies(root: &Path) -> Result<Vec<PluginDependencyCheck>> {
    let manifest = crate::plugins::load_plugin_manifest(root)?;
    let (outcome, _) = crate::plugins::mcp::load_plugin_mcp_declarations(&manifest)?;
    plugin_install::dependencies::inspect_plugin_dependencies(
        &manifest, outcome,
        super::skill_roots::skill_roots,
        |root| super::skill_roots::skill_files(root, super::skill_roots::package_format(&manifest)),
        super::agents::load_metadata,
        super::mcp_portable::env_names_match,
    )
}

pub(crate) fn missing_mcp_executable(server: &McpServer) -> Result<Option<PluginDependencyCheck>> {
    plugin_install::dependencies::missing_mcp_executable(
        server, super::mcp_environment::runtime_cwd, super::mcp_portable::env_names_match,
    )
}
