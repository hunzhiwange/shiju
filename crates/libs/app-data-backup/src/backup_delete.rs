//! Delete exactly one same-scope local archive, while respecting open sources.
use super::{
    backup_create::LocalBackup,
    backup_create_files as files,
    safety_snapshot::{MAX_SNAPSHOT_BYTES, SafetySnapshotError, SafetySnapshotScope},
};
use std::{io::Read, os::fd::AsRawFd, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupDeleteError {
    NotFound,
    Protected,
    Snapshot(SafetySnapshotError),
}
impl From<SafetySnapshotError> for BackupDeleteError {
    fn from(error: SafetySnapshotError) -> Self {
        match error {
            SafetySnapshotError::Conflict => Self::Protected,
            error => Self::Snapshot(error),
        }
    }
}
impl std::fmt::Display for BackupDeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "backup deletion: {self:?}")
    }
}
impl std::error::Error for BackupDeleteError {}

/// Authorize the executor-owned root and current scope before calling. An open
/// `LocalBackup` protects a restore source or rollback point until it is dropped.
/// All archive mutations must use this descriptor/lock protocol. No request path
/// is accepted. Corrupt archives without identifiable ownership fail closed.
pub fn delete_local_backup(
    root: &Path,
    scope: &SafetySnapshotScope,
    id: &str,
) -> Result<(), BackupDeleteError> {
    let name = files::archive_name(id)?;
    let directory = files::open_root(root)?;
    // Probe existence without following links, to distinguish a missing ID from
    // unsafe storage. The actual open independently checks type/owner/mode/links.
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live directory, valid C string and writable stat allocation.
    if unsafe {
        libc::fstatat(directory.as_raw_fd(), name.as_ptr(), metadata.as_mut_ptr(), libc::AT_SYMLINK_NOFOLLOW)
    } != 0 {
        return Err(if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            BackupDeleteError::NotFound
        } else {
            BackupDeleteError::Snapshot(SafetySnapshotError::StorageUnavailable)
        });
    }
    let mut file = files::open_backup(&directory, id)?;
    files::lock_backup(&file, true)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_SNAPSHOT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| SafetySnapshotError::StorageUnavailable)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SafetySnapshotError::TooLarge.into());
    }
    let backup: LocalBackup = serde_json::from_slice(&bytes).map_err(|_| SafetySnapshotError::Corrupt)?;
    if backup.scope() != scope {
        return Err(SafetySnapshotError::ScopeMismatch.into());
    }
    if scope.host_id.trim().is_empty() || scope.principal_id.trim().is_empty() {
        return Err(SafetySnapshotError::InvalidSelection.into());
    }
    // SAFETY: validated single component relative to the pinned private root.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(SafetySnapshotError::StorageUnavailable.into());
    }
    directory.sync_all().map_err(|_| SafetySnapshotError::StorageUnavailable)?;
    Ok(())
}
