//! Restore orchestration for authorized owner bundles, never whole databases.
use std::path::Path;
use vw_api_types::agent_apps::{DataManagementTarget, DataOperationResult};
use super::{
    backup_create::{LocalBackupReceipt, open_local_backup},
    backup_delete::BackupDeleteError,
    post_import::{Projection, refresh_after_commit},
    retention::{RetentionPolicy, RetentionReport, cleanup_local_backups},
    safety_snapshot::{OwnerPreimage, SafetySnapshotError, SafetySnapshotReceipt,
        SafetySnapshotScope, create_safety_snapshot, open_safety_snapshot},
};

/// Returned even after a possibly partial owner write. Persist the receipt and
/// deny further scope writes until a failed restore has been recovered by owners.
pub struct RestoreReport<E> {
    pub safety_point: SafetySnapshotReceipt,
    pub restored: Result<(), E>,
    pub projections: Option<DataOperationResult>,
    pub retention: Option<Result<RetentionReport, BackupDeleteError>>,
}

/// The caller holds the same authorized target exclusion guard used for imports
/// and deletion, plus owner transaction/version guards, throughout this call.
/// `validate` must check ALL owner encodings, versions and references read-only;
/// unsupported owners fail there, before capture or writes. `capture` returns the
/// exact current bundles (including absence). `restore` uses existing owners,
/// preserves dependency ordering and rejects version races; it is never SQL.
/// Persist the safety receipt in the owning task before allowing the first write.
/// A failed persistence callback prevents writes but leaves the durable snapshot.
/// Roots are executor-owned private directories, never request paths. Safety
/// snapshots use task 8's separate store, so backup retention cannot prune them.
#[allow(clippy::too_many_arguments)]
pub fn restore_local_backup<E>(
    backup_root: &Path,
    safety_root: &Path,
    source: &LocalBackupReceipt,
    target: &DataManagementTarget,
    policy: RetentionPolicy,
    validate: impl FnOnce(&[OwnerPreimage]) -> Result<(), SafetySnapshotError>,
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
    persist: impl FnOnce(&SafetySnapshotReceipt) -> Result<(), SafetySnapshotError>,
    restore: impl FnOnce(&[OwnerPreimage]) -> Result<(), E>,
    refresh: impl FnMut(&DataManagementTarget, Projection) -> Result<(), ()>,
) -> Result<RestoreReport<E>, SafetySnapshotError> {
    let scope = SafetySnapshotScope {
        project: target.scope.clone(),
        principal_id: target.principal_id.clone(),
        host_id: target.target_host_id.clone(),
    };
    // Keep the opened source alive through owner writes, projections and pruning.
    let backup = open_local_backup(backup_root, source, &scope)?;
    validate(backup.preimages())?;
    let targets = backup.preimages().iter().map(|image| image.target.clone()).collect::<Vec<_>>();
    let safety_point = create_safety_snapshot(safety_root, &scope, &targets, capture)?;
    // Independently read back exact target coverage before any owner write.
    let _current = open_safety_snapshot(safety_root, &safety_point, &scope, &targets)?;
    persist(&safety_point)?;
    let restored = restore(backup.preimages());
    let (projections, retention) = if restored.is_ok() {
        (Some(refresh_after_commit(target, refresh)),
         Some(cleanup_local_backups(backup_root, &scope, policy)))
    } else {
        // A failed owner may have written: preserve every recovery artifact.
        (None, None)
    };
    drop(backup);
    Ok(RestoreReport { safety_point, restored, projections, retention })
}
