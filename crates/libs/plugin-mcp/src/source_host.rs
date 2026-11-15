//! Audited source reads supplied by the plugin package owner.
use std::path::{Path, PathBuf};
use anyhow::Result;
use plugin_types::PluginManifest;
use super::mcp_legacy::PluginMcpParseOutcome;
pub trait SourceHost: Send + Sync {
    fn load_manifest(&self, root: &Path) -> Result<PluginManifest>;
    fn load_declarations(&self, manifest: &PluginManifest) -> Result<(PluginMcpParseOutcome, PathBuf)>;
}
