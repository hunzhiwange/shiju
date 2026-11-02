//! Cross-process lifecycle exclusion and deterministic upgrade rollback journal.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    pub key: String,
    pub transaction: String,
    pub committed: bool,
}
fn journal_path(root: &Path) -> PathBuf {
    root.join(".plugin-upgrade.json")
}
fn regular(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("plugin state must be a regular file");
    }
    Ok(())
}
pub(super) fn key_valid(key: &str) -> bool {
    !key.trim().is_empty() && key != "." && !key.contains("..") && !key.contains(['/', '\\', '\0'])
}

/// Lock lives beside the store so installing a lock never changes package contents.
pub(crate) fn lock(root: &Path) -> Result<Option<File>> {
    if !root.exists() {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("plugin store must be a directory, not a symlink");
    }
    let canonical = root.canonicalize()?;
    let name = canonical
        .file_name()
        .context("plugin store requires a directory name")?
        .to_str()
        .context("non UTF-8 plugin store name")?;
    let path = canonical.with_file_name(format!(".{name}.lifecycle.lock"));
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(&path).context("failed to open plugin lifecycle lock")?;
    regular(&path)?;
    file.lock().context("failed to lock plugin lifecycle")?;
    recover(&canonical)?;
    Ok(Some(file))
}
pub(super) fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Directory handles require FILE_FLAG_BACKUP_SEMANTICS; flush failures remain visible.
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(0x02000000)
            .open(path)?
            .sync_all()?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        bail!("durable plugin directory sync is unsupported on this platform");
    }
    Ok(())
}
pub(super) fn sync_tree(root: &Path) -> Result<()> {
    for item in fs::read_dir(root)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            sync_tree(&item.path())?;
        } else if item.file_type()?.is_file() {
            #[cfg(windows)]
            OpenOptions::new().read(true).write(true).open(item.path())?.sync_all()?;
            #[cfg(not(windows))]
            File::open(item.path())?.sync_all()?;
        } else {
            bail!("unsafe plugin transaction entry");
        }
    }
    sync_dir(root)
}
pub(super) fn write_journal(root: &Path, journal: &Journal, initial: bool) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(&serde_json::to_vec(journal)?)?;
    file.as_file().sync_all()?;
    if initial {
        file.persist_noclobber(journal_path(root))?;
    } else {
        regular(&journal_path(root))?;
        file.persist(journal_path(root))?;
    }
    sync_dir(root)
}
fn move_synced(from: &Path, to: &Path) -> Result<()> {
    fs::rename(from, to)?;
    sync_dir(from.parent().context("missing transaction parent")?)?;
    sync_dir(to.parent().context("missing transaction parent")?)
}
pub(super) fn rename(from: &Path, to: &Path) -> Result<()> {
    move_synced(from, to)
}
/// Prepared journal always rolls back. Committed journal only removes its owned backup.
pub(super) fn recover(root: &Path) -> Result<()> {
    let path = journal_path(root);
    if !path.try_exists()? {
        return Ok(());
    }
    regular(&path)?;
    if fs::metadata(&path)?.len() > 4096 {
        bail!("oversized plugin transaction journal");
    }
    let journal: Journal = serde_json::from_slice(&fs::read(&path)?)?;
    if !key_valid(&journal.key)
        || !journal.transaction.starts_with(".plugin-upgrade-")
        || journal.transaction.contains(['/', '\\'])
        || journal.transaction.contains("..")
    {
        bail!("invalid plugin transaction journal");
    }
    let transaction = root.join(&journal.transaction);
    if fs::symlink_metadata(&transaction)?.file_type().is_symlink() {
        bail!("unsafe plugin transaction directory");
    }
    let old = transaction.join("old");
    let old_metadata = fs::symlink_metadata(&old)?;
    if !old_metadata.is_dir() || old_metadata.file_type().is_symlink() {
        bail!("unsafe plugin rollback directory");
    }
    let destination = root.join(&journal.key);
    let metadata = super::identity::metadata_path(&destination)?;
    let backup = old.join(&journal.key);
    let backup_metadata = super::identity::metadata_path(&backup)?;
    if !journal.committed {
        if backup.try_exists()? {
            let metadata = fs::symlink_metadata(&backup)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                bail!("unsafe plugin rollback package");
            }
            if destination.try_exists()? {
                fs::remove_dir_all(&destination)?;
            }
            move_synced(&backup, &destination)?;
        }
        if backup_metadata.try_exists()? {
            if metadata.try_exists()? {
                fs::remove_file(&metadata)?;
            }
            move_synced(&backup_metadata, &metadata)?;
        }
    }
    if journal.committed {
        let manifest = crate::plugins::load_plugin_manifest(&destination)?;
        super::identity::read_install_metadata(&destination, &manifest)?
            .context("committed upgrade metadata missing")?;
    }
    fs::remove_file(&path)?;
    sync_dir(root)?;
    fs::remove_dir_all(transaction)?;
    sync_dir(root)
}
