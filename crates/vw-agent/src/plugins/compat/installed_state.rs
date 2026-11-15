//! Host manifest loading is injected into lifecycle recovery.
pub use plugin_install::installed_state::{Journal, key_valid, rename, sync_dir, sync_tree, write_journal};
use anyhow::Result;
use std::{fs::File, path::Path};
pub(crate) fn lock(root: &Path) -> Result<Option<File>> {
    plugin_install::installed_state::lock(root, crate::plugins::load_plugin_manifest)
}
pub(crate) fn read_lock(root: &Path) -> Result<Option<File>> {
    plugin_install::installed_state::read_lock(root, crate::plugins::load_plugin_manifest)
}
pub(super) fn recover(root: &Path) -> Result<()> {
    plugin_install::installed_state::recover(root, crate::plugins::load_plugin_manifest)
}
