//! Explicit audited preview and journaled activation. No package is changed at preview time.
use super::{
    identity::{metadata_path, read_install_metadata},
    installed_state as state,
};
use crate::plugins::{
    PluginIdentity, install_plugin_from_marketplace, install_plugin_from_source,
    load_plugin_manifest,
};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct PluginUpgradePreview {
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    root: PathBuf,
    identity: PluginIdentity,
    expected_hash: String,
    expected_metadata: Vec<u8>,
    new_hash: String,
    staging: tempfile::TempDir,
}
#[cfg(test)]
thread_local! {pub(super) static FAIL_ACTIVATION: std::cell::Cell<bool> = const {std::cell::Cell::new(false)};}
fn hash(root: &Path) -> Result<String> {
    Ok(super::hook_trust_content::plugin_content_hash(root, &|| false)?)
}
fn identity(root: &Path, key: &str) -> Result<PluginIdentity> {
    if !state::key_valid(key) {
        bail!("invalid plugin upgrade identity");
    }
    let path = root.join(key);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("installed plugin must be a directory, not a symlink");
    }
    let manifest = load_plugin_manifest(&path)?;
    let identity = read_install_metadata(&path, &manifest)?
        .context("upgrade requires installation metadata")?
        .identity;
    if identity.as_key() != key {
        bail!("plugin upgrade identity mismatch");
    }
    Ok(identity)
}
pub fn prepare_plugin_marketplace_upgrade(
    entry: &crate::plugins::MarketplaceEntry,
    plugins: &Path,
) -> Result<PluginUpgradePreview> {
    prepare_with(plugins, &entry.identity.as_key(), |_, root| {
        crate::plugins::install_plugin_marketplace_entry(entry, root)
    })
}
pub fn prepare_plugin_upgrade(
    source: &str,
    plugins: &Path,
    key: &str,
) -> Result<PluginUpgradePreview> {
    prepare_with(plugins, key, |identity, root| match identity.marketplace_name() {
        Some(market) => install_plugin_from_marketplace(source, root, market),
        None => install_plugin_from_source(source, root),
    })
}
fn prepare_with(
    plugins: &Path,
    key: &str,
    materialize: impl FnOnce(&PluginIdentity, &Path) -> Result<crate::plugins::PluginInstallResult>,
) -> Result<PluginUpgradePreview> {
    if !state::key_valid(key) {
        bail!("invalid plugin upgrade identity");
    }
    let _guard = state::lock(plugins)?;
    let root = plugins.canonicalize()?;
    let identity = identity(&root, key)?;
    let old = root.join(key);
    let old_version = load_plugin_manifest(&old)?.version;
    let expected_hash = hash(&old)?;
    let expected_metadata = fs::read(metadata_path(&old)?)?;
    let staging = tempfile::Builder::new().prefix(".plugin-upgrade-").tempdir_in(&root)?;
    let new_root = staging.path().join("new");
    let installed = materialize(&identity, &new_root)?;
    if installed.identity != identity {
        bail!("plugin upgrade must retain exact identity");
    }
    let new_hash = hash(&installed.installed_dir)?;
    state::sync_tree(staging.path())?;
    Ok(PluginUpgradePreview {
        old_version,
        new_version: installed.manifest.version,
        root,
        identity,
        expected_hash,
        expected_metadata,
        new_hash,
        staging,
    })
}
pub fn activate_plugin_upgrade(preview: PluginUpgradePreview) -> Result<()> {
    let _guard = state::lock(&preview.root)?;
    let key = preview.identity.as_key();
    let destination = preview.root.join(&key);
    let staged = preview.staging.path().join("new").join(&key);
    if identity(&preview.root, &key)? != preview.identity
        || hash(&destination)? != preview.expected_hash
        || fs::read(metadata_path(&destination)?)? != preview.expected_metadata
    {
        bail!("installed plugin changed after upgrade preview");
    }
    if hash(&staged)? != preview.new_hash {
        bail!("staged plugin changed after upgrade preview");
    }
    // Re-audit the staged metadata immediately before committing to disk.
    identity(&preview.staging.path().join("new"), &key)?;
    fs::create_dir(preview.staging.path().join("old"))?;
    state::sync_tree(preview.staging.path())?;
    let transaction = preview
        .staging
        .path()
        .file_name()
        .context("missing upgrade staging name")?
        .to_str()
        .context("non UTF-8 staging name")?
        .to_owned();
    let mut journal = state::Journal { key: key.clone(), transaction, committed: false };
    state::write_journal(&preview.root, &journal, true)?;
    // Once journaled, recovery owns this directory even if activation or rollback fails.
    let staging = preview.staging.keep();
    crate::plugins::compat::mcp_connection_sessions::close_plugin(&destination);
    let result = (|| {
        let backup = staging.join("old").join(&key);
        state::rename(&destination, &backup)?;
        state::rename(&metadata_path(&destination)?, &metadata_path(&backup)?)?;
        state::rename(&staged, &destination)?;
        #[cfg(test)]
        if FAIL_ACTIVATION.with(|flag| flag.replace(false)) {
            bail!("injected plugin activation failure");
        }
        state::rename(&metadata_path(&staged)?, &metadata_path(&destination)?)?;
        identity(&preview.root, &key)?;
        journal.committed = true;
        state::write_journal(&preview.root, &journal, false)
    })();
    match result {
        Ok(()) => {
            if let Err(error) = state::recover(&preview.root) {
                tracing::warn!(%error, "committed plugin upgrade cleanup deferred");
            }
            Ok(())
        }
        Err(error) => {
            journal.committed = false;
            state::write_journal(&preview.root, &journal, false)
                .with_context(|| format!("{error:#}; failed to persist rollback intent"))?;
            state::recover(&preview.root)
                .with_context(|| format!("{error:#}; upgrade rollback failed"))?;
            Err(error)
        }
    }
}
