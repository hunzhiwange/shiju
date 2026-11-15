//! Backend due checks for authorized host-local backups.
use super::{
    backup_api::lock_directory,
    backup_create::LocalBackupReceipt,
    backup_list::{BackupRestoreStatus, list_local_backups},
    safety_snapshot::{SafetySnapshotError, SafetySnapshotScope},
};
use std::path::Path;

/// Zero disables backup creation before any filesystem access. The directory
/// lock covers due-check and publication, including competing gateway processes.
/// Use archive timestamps so gateway restarts do not reset the schedule.
pub fn periodic_backup_if_needed<E: From<SafetySnapshotError>>(
    root: &Path,
    scope: &SafetySnapshotScope,
    interval_hours: u32,
    now_ms: i64,
    capture: impl FnOnce() -> Result<Option<LocalBackupReceipt>, E>,
) -> Result<Option<LocalBackupReceipt>, E> {
    if interval_hours == 0 {
        return Ok(None);
    }
    if now_ms < 0 {
        return Err(SafetySnapshotError::InvalidSelection.into());
    }
    let _guard = lock_directory(root)?;
    let latest = list_local_backups(root, scope)?
        .into_iter()
        .filter(|entry| entry.restore_status == BackupRestoreStatus::RequiresOwnerValidation)
        .map(|entry| entry.created_at_ms)
        .max();
    let interval_ms = i64::from(interval_hours) * 3_600_000;
    if latest.is_some_and(|last| now_ms.saturating_sub(last) < interval_ms) {
        return Ok(None);
    }
    capture()
}
