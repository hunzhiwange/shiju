//! Host manifest schema and root auditing remain explicit inputs.
use anyhow::Result;
use std::path::Path;
pub(crate) fn is_plugin_candidate(root: &Path) -> Result<bool> {
    plugin_install::discovery::is_plugin_candidate(root, super::portable_manifest::SCHEMA_PREFIX, super::super::audit::canonical_plugin_root)
}
