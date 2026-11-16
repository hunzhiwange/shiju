//! Compose audited manifest/MCP loading and the existing skill inventory.
use agent_client_protocol::McpServer;
use anyhow::Result;
use std::path::Path;
pub use plugin_install::dependencies::{DependencyState, PluginDependencyCheck};

pub fn inspect_plugin_dependencies(root: &Path) -> Result<Vec<PluginDependencyCheck>> {
    let manifest = crate::load_plugin_manifest(root)?;
    let (outcome, _) = crate::mcp::load_plugin_mcp_declarations(&manifest)?;
    plugin_install::dependencies::inspect_plugin_dependencies(
        &manifest, outcome,
        |manifest| plugin_install::skill_roots::skill_roots(manifest, &crate::audit::resolve_component_path),
        |root| plugin_install::skill_roots::skill_files(root, plugin_install::skill_roots::package_format(&manifest)),
        |root, skill| plugin_install::agents::load_metadata(root, skill, crate::audit::resolve_component_path),
        plugin_mcp::mcp_portable::env_names_match,
    )
}

pub fn missing_mcp_executable(server: &McpServer) -> Result<Option<PluginDependencyCheck>> {
    plugin_install::dependencies::missing_mcp_executable(
        server, plugin_mcp::mcp_environment::runtime_cwd, plugin_mcp::mcp_portable::env_names_match,
    )
}
