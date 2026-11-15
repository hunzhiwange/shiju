//! 宿主仅组合已有路径审计和清单读取；覆盖业务由 plugin-mcp 持有。
use anyhow::Result;
use std::path::Path;
use plugin_types::{PluginManifest, PluginPath};
use super::mcp_legacy::PluginMcpParseOutcome;

struct OverlaySource;
impl plugin_mcp::mcp_portable_overlay::OverlayHost for OverlaySource {
    fn resolve_component_path(&self, root: &Path, field: &str, raw: &str) -> Result<PluginPath> {
        super::super::audit::resolve_component_path(root, field, raw)
    }
    fn parse_legacy_manifest(&self, root: &Path, path: &Path, bytes: &[u8]) -> Result<PluginManifest> {
        super::legacy_manifest::parse_legacy_manifest(root, path, bytes)
    }
}

pub(crate) fn apply_env_overlay(root: &Path, outcome: &mut PluginMcpParseOutcome) -> Result<()> {
    plugin_mcp::mcp_portable_overlay::apply_env_overlay(&OverlaySource, root, outcome)
}
