//! Audited package reads bridge to the MCP credential implementation.
use crate::plugins::load_plugin_manifest;
use agent_client_protocol::McpServerHttp;
use anyhow::Result;
use std::path::{Path, PathBuf};
use plugin_mcp::mcp_legacy::PluginMcpParseOutcome;
pub use plugin_mcp::mcp_auth::HttpContext;
pub(crate) use plugin_mcp::mcp_auth::bind;
pub(super) struct Source;
impl plugin_mcp::source_host::SourceHost for Source {
    fn load_manifest(&self, root: &Path) -> Result<plugin_types::PluginManifest> { load_plugin_manifest(root) }
    fn load_declarations(&self, manifest: &plugin_types::PluginManifest) -> Result<(PluginMcpParseOutcome, PathBuf)> {
        crate::plugins::mcp::load_plugin_mcp_declarations(manifest)
    }
}
pub(crate) fn prepare(outcome: &mut PluginMcpParseOutcome, plugin: &DiscoveredPlugin, names: &HashMap<String, String>) -> Result<HashMap<String, HttpContext>> {
    plugin_mcp::mcp_auth::prepare(outcome, &plugin.manifest, &plugin.source_dir, &plugin.identity, names)
}
pub(super) fn source_config(server: &McpServerHttp) -> Result<Option<serde_json::Map<String, serde_json::Value>>> {
    plugin_mcp::mcp_auth::source_config(&Source, server)
}
pub async fn discover_oauth(server: &McpServerHttp) -> Result<super::mcp_oauth_discovery::OAuthDiscovery> {
    plugin_mcp::mcp_auth::discover_oauth(&Source, server).await
}
pub(crate) fn runtime_headers(server: &McpServerHttp) -> Result<Option<reqwest_012::header::HeaderMap>> {
    plugin_mcp::mcp_auth::runtime_headers(&Source, server)
}
pub(crate) fn runtime_oauth_headers(server: &McpServerHttp, token: Option<&str>) -> Result<Option<reqwest_012::header::HeaderMap>> {
    plugin_mcp::mcp_auth::runtime_oauth_headers(&Source, server, token)
}
pub(crate) fn hook_source_matches(server: &McpServerHttp, root: &Path, name: &str) -> Result<bool> {
    plugin_mcp::mcp_auth::hook_source_matches(&Source, server, root, name)
}
