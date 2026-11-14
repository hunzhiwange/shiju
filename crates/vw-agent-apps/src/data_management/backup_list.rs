//! Scope-filtered metadata for the existing host-local backup format.
use super::{
    backup_create::{self, LocalBackup},
    backup_create_files as files,
    safety_snapshot::{MAX_SNAPSHOT_BYTES, SafetySnapshotError as Error, SafetySnapshotScope},
};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
use vw_api_types::agent_apps::data_management::DataArtifactKind;

/// Structural eligibility only. Restore must separately authorize, verify its
/// retained receipt digest and validate every owner's version/capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupRestoreStatus {
    RequiresOwnerValidation,
    VersionUnsupported,
    Corrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalBackupMetadata {
    pub id: String,
    pub kind: DataArtifactKind,
    pub format_version: u32,
    pub size_bytes: u64,
    pub created_at_ms: i64,
    pub restore_status: BackupRestoreStatus,
}

/// Callers supply an authorized executor-owned root and full current identity.
/// Unreadable, unsafe or unparseable archives are omitted: their ownership cannot
/// be established safely. A parseable same-scope archive has an explicit status.
/// No preimage, label, native path or foreign archive diagnostic is returned.
pub fn list_local_backups(
    root: &Path,
    scope: &SafetySnapshotScope,
) -> Result<Vec<LocalBackupMetadata>, Error> {
    if scope.host_id.trim().is_empty() || scope.principal_id.trim().is_empty() {
        return Err(Error::InvalidSelection);
    }
    let directory = files::open_root(root)?;
    let mut entries = Vec::new();
    for id in files::backup_ids(&directory)? {
        let mut file = match files::open_backup(&directory, &id) {
            Ok(file) => file,
            Err(Error::StorageUnavailable | Error::UnsafeStorage | Error::TooLarge) => continue,
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_SNAPSHOT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::StorageUnavailable)?;
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            continue;
        }
        let Ok(backup) = serde_json::from_slice::<LocalBackup>(&bytes) else { continue };
        if backup.scope() != scope {
            continue;
        }
        let restore_status = match backup_create::validate(&backup, scope) {
            Ok(()) if backup.created_at_ms() >= 0 => BackupRestoreStatus::RequiresOwnerValidation,
            Err(Error::VersionUnsupported) => BackupRestoreStatus::VersionUnsupported,
            _ => BackupRestoreStatus::Corrupt,
        };
        entries.push(LocalBackupMetadata {
            id,
            kind: backup.kind,
            format_version: backup.format_version,
            size_bytes: bytes.len() as u64,
            created_at_ms: backup.created_at_ms(),
            restore_status,
        });
    }
    entries.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms).then(a.id.cmp(&b.id)));
    Ok(entries)
}
