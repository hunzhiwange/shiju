//! Metadata and label operations for the existing local archive format.
use super::{
    backup_create::{self, LocalBackup, LocalBackupReceipt},
    backup_create_files as files,
    safety_snapshot::{
        MAX_SNAPSHOT_BYTES, SafetySnapshotError as Error, SafetySnapshotScope, SnapshotCategory,
    },
};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, os::fd::AsRawFd, path::Path};
use vw_api_types::agent_apps::data_management::*;

/// Serializes API operations across processes without inventing another store.
/// Keep alive through capture, publication and response metadata readback.
pub struct BackupDirectoryGuard {
    _directory: File,
}
pub fn lock_directory(root: &Path) -> Result<BackupDirectoryGuard, Error> {
    let directory = files::open_root(root)?;
    // SAFETY: owned live directory descriptor; closing releases the lock.
    if unsafe { libc::flock(directory.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(Error::Conflict);
    }
    Ok(BackupDirectoryGuard { _directory: directory })
}
fn read(
    root: &Path,
    scope: &SafetySnapshotScope,
    id: &str,
    exclusive: bool,
) -> Result<(File, LocalBackup, LocalBackupReceipt, usize), Error> {
    let mut file = files::open_backup(&files::open_root(root)?, id)?;
    files::lock_backup(&file, exclusive)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_SNAPSHOT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::StorageUnavailable)?;
    let receipt: LocalBackupReceipt = serde_json::from_value(serde_json::json!({
        "id": id, "digest": <[u8; 32]>::from(Sha256::digest(&bytes))
    }))
    .map_err(|_| Error::Corrupt)?;
    let backup = backup_create::decode(&bytes, &receipt, scope)?;
    Ok((file, backup, receipt, bytes.len()))
}
pub fn summary(
    root: &Path,
    scope: &SafetySnapshotScope,
    id: &str,
) -> Result<DataBackupSummary, Error> {
    let (_file, backup, receipt, size) = read(root, scope, id, false)?;
    let mut categories = Vec::new();
    for image in backup.preimages() {
        let category = match image.target.category {
            SnapshotCategory::Provider => DataCategory::Providers,
            SnapshotCategory::Mcp => DataCategory::Mcp,
            SnapshotCategory::Skill => DataCategory::Skills,
            SnapshotCategory::Prompt => DataCategory::Rules,
            SnapshotCategory::Profile => DataCategory::Profiles,
            SnapshotCategory::Preference => DataCategory::Preferences,
            SnapshotCategory::Usage => DataCategory::Usage,
        };
        if !categories.contains(&category) {
            categories.push(category);
        }
    }
    let digest = digest_string(receipt.digest());
    Ok(DataBackupSummary {
        backup_id: DataBackupId(id.into()),
        target: DataManagementTarget {
            scope: scope.project.clone(),
            target_host_id: scope.host_id.clone(),
            principal_id: scope.principal_id.clone(),
        },
        name: backup.name().into(),
        revision: digest.clone(),
        digest_sha256: digest,
        kind: DataArtifactKind::LocalRollback,
        format_version: 1,
        created_at_ms: backup.created_at_ms(),
        size_bytes: size as u64,
        categories,
    })
}
/// Display labels never become filenames; ID and owner bytes remain unchanged.
pub fn rename_label(
    root: &Path,
    scope: &SafetySnapshotScope,
    id: &str,
    revision: &str,
    name: &str,
) -> Result<(), Error> {
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(Error::InvalidSelection);
    }
    let (_file, mut backup, receipt, _) = read(root, scope, id, true)?;
    if digest_string(receipt.digest()) != revision {
        return Err(Error::Conflict);
    }
    backup.set_name(name.to_owned());
    let bytes = serde_json::to_vec(&backup).map_err(|_| Error::Corrupt)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(Error::TooLarge);
    }
    files::replace(&files::open_root(root)?, id, &bytes)
}

fn digest_string(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Revision check and deletion share the same source lock.
pub fn delete_checked(
    root: &Path,
    scope: &SafetySnapshotScope,
    id: &str,
    revision: &str,
) -> Result<(), Error> {
    let (_file, _backup, receipt, _) = read(root, scope, id, true)?;
    if digest_string(receipt.digest()) != revision {
        return Err(Error::Conflict);
    }
    let directory = files::open_root(root)?;
    let name = files::archive_name(id)?;
    // SAFETY: validated basename and owned private directory descriptor.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(Error::StorageUnavailable);
    }
    directory.sync_all().map_err(|_| Error::StorageUnavailable)
}
