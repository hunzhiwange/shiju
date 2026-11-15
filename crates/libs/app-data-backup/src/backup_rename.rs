//! Rename a scoped host-local archive without rewriting its recovery contents.
use super::{
    backup_create::{self, LocalBackupReceipt},
    backup_create_files as files,
    safety_snapshot::{MAX_SNAPSHOT_BYTES, SafetySnapshotError as Error, SafetySnapshotScope},
};
use std::{io::Read, path::Path};

/// Authorize the executor-owned root and scope before calling. Target is an ID
/// (lowercase ASCII letters, digits and hyphens), never a path or display label.
/// The returned receipt replaces the old ID while retaining its content digest.
/// A directory sync failure can occur after the rename has committed; callers
/// must reconcile the destination rather than blindly retry the old ID.
pub fn rename_local_backup(
    root: &Path,
    receipt: &LocalBackupReceipt,
    scope: &SafetySnapshotScope,
    target_id: &str,
) -> Result<LocalBackupReceipt, Error> {
    let directory = files::open_root(root)?;
    let mut file = files::open_backup(&directory, receipt.id())?;
    files::lock_backup(&file, true)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(MAX_SNAPSHOT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::StorageUnavailable)?;
    backup_create::decode(&bytes, receipt, scope)?;
    files::rename_backup(&directory, receipt.id(), target_id)?;
    Ok(receipt.renamed(target_id.to_owned()))
}
