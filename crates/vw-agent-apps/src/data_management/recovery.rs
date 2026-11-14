//! Non-destructive diagnostics independent of opening a healthy target store.
//!
//! Executors use their normal diagnostic channel even when an owner cannot open
//! its database. Never initialize, migrate, repair or replace a store in a probe.
use std::path::Path;

use super::{
    backup_list::{LocalBackupMetadata, list_local_backups},
    backup_restore::RestoreReport,
    safety_snapshot::{SafetySnapshotError, SafetySnapshotScope},
};

/// The controlled write entry point validates all backup owners before capture,
/// persists the safety receipt before writes and preserves it on failure. A
/// damaged target whose exact preimages cannot be captured must fail closed;
/// callers must not substitute an empty capture or rebuild its database.
pub use super::backup_restore::restore_local_backup as recover_from_backup;

/// Stable owner diagnostics; do not return database errors or sensitive payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetHealth {
    Healthy,
    Corrupt,
    VersionUnsupported,
    Unavailable,
}

pub struct RecoveryDiagnosis {
    pub target: TargetHealth,
    /// Independent of target health. Listing failure must remain distinguishable
    /// from a healthy, empty backup store. Entries still require owner validation.
    pub backups: Result<Vec<LocalBackupMetadata>, SafetySnapshotError>,
}

/// Supply an authorized executor-owned root and full identity. The owner probe
/// must be read-only (including opening without CREATE or schema migration), and
/// classify unsupported future schemas without attempting a downgrade. Listing
/// uses only the separate backup store, never the damaged target connection.
/// Original database, WAL and SHM evidence remain in place for normal diagnostics.
pub fn diagnose_recovery(
    backup_root: &Path,
    scope: &SafetySnapshotScope,
    probe: impl FnOnce() -> TargetHealth,
) -> Result<RecoveryDiagnosis, SafetySnapshotError> {
    if scope.principal_id.trim().is_empty() || scope.host_id.trim().is_empty() {
        return Err(SafetySnapshotError::InvalidSelection);
    }
    let target = probe();
    let backups = list_local_backups(backup_root, scope);
    Ok(RecoveryDiagnosis { target, backups })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryState {
    Restored,
    RecoveryRequired,
}

impl<E> RestoreReport<E> {
    /// A failed owner may already have written. Persist this state together with
    /// the safety receipt and block scope writes until owner recovery succeeds.
    /// Projection errors are reported separately; they do not undo owner commit.
    pub fn recovery_state(&self) -> RecoveryState {
        match &self.restored {
            Ok(()) => RecoveryState::Restored,
            Err(_) => RecoveryState::RecoveryRequired,
        }
    }
}
