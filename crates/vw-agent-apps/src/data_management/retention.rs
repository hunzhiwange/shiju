//! Count-based pruning of authorized host-local archives.
use super::{
    backup_delete::{BackupDeleteError, delete_local_backup},
    backup_list::{BackupRestoreStatus, list_local_backups},
    safety_snapshot::{SafetySnapshotError, SafetySnapshotScope},
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    retain: usize,
}
impl RetentionPolicy {
    /// Source settings are u32: zero means the minimum (one), never delete all.
    /// Reject malformed signed input before touching the backup directory.
    pub fn new(count: i64) -> Result<Self, SafetySnapshotError> {
        let count = u32::try_from(count).map_err(|_| SafetySnapshotError::InvalidSelection)?;
        Ok(Self { retain: count.max(1) as usize })
    }
}
impl Default for RetentionPolicy {
    fn default() -> Self {
        Self { retain: 10 }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RetentionReport {
    pub deleted: Vec<String>,
    pub protected: Vec<String>,
}

/// Authorize root and full identity first. Open restore sources and rollback
/// points must retain their LocalBackup guards throughout cleanup. File locks
/// protect inode identity, including case-equivalent names on macOS volumes.
/// Invalid/unsupported archives are never candidates or counted toward retention.
/// Errors stop cleanup; earlier successful deletions remain committed. Retrying
/// re-enumerates the directory and is safe. This is not an atomic batch operation.
pub fn cleanup_local_backups(
    root: &Path,
    scope: &SafetySnapshotScope,
    policy: RetentionPolicy,
) -> Result<RetentionReport, BackupDeleteError> {
    let mut entries = list_local_backups(root, scope)?;
    entries.retain(|entry| entry.restore_status == BackupRestoreStatus::RequiresOwnerValidation);
    let remove_count = entries.len().saturating_sub(policy.retain);
    // Oldest first; equal timestamps use ascending opaque ID, independent of
    // filesystem enumeration order.
    entries.sort_by(|a, b| a.created_at_ms.cmp(&b.created_at_ms).then(a.id.cmp(&b.id)));
    let mut report = RetentionReport::default();
    let mut removed = 0;
    for entry in entries {
        if removed >= remove_count {
            break;
        }
        match delete_local_backup(root, scope, &entry.id) {
            Ok(()) => {
                removed += 1;
                report.deleted.push(entry.id);
            }
            Err(BackupDeleteError::Protected) => report.protected.push(entry.id),
            // A concurrent cleanup already removed this candidate.
            Err(BackupDeleteError::NotFound) => removed += 1,
            Err(error) => return Err(error),
        }
    }
    Ok(report)
}
