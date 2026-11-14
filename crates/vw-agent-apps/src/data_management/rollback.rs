//! Failure recovery for selected-category dispatch using an existing durable
//! preimage archive. Owners retain locks for this entire call, or restore with
//! CAS against their operation's written revisions (including partial failures).
use std::{collections::BTreeSet, path::Path};

use super::{
    apply::{ApplyError, ApplyReport, apply_selected},
    safety_snapshot::{
        OwnerPreimage, SafetySnapshotError, SafetySnapshotReceipt, SafetySnapshotScope,
        SnapshotCategory, SnapshotTarget, open_safety_snapshot,
    },
    source::{SourceCategory, SourceScope, SourceSnapshot, SourceSummary},
};

pub struct RollbackReport<E, R> {
    pub receipt: SafetySnapshotReceipt,
    pub apply: ApplyReport<E>,
    pub restored: Vec<SourceCategory>,
    /// Nonempty means recovery is required. Persist these categories and the
    /// receipt with the task; deny further writes in its scope until repaired.
    pub recovery_required: Vec<(SourceCategory, R)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackError {
    Snapshot(SafetySnapshotError),
    Apply(ApplyError),
    InvalidSelection,
}

fn category(value: SnapshotCategory) -> SourceCategory {
    match value {
        SnapshotCategory::Provider => SourceCategory::Providers,
        SnapshotCategory::Mcp => SourceCategory::Mcp,
        SnapshotCategory::Skill => SourceCategory::Skills,
        SnapshotCategory::Prompt => SourceCategory::Rules,
        SnapshotCategory::Profile => SourceCategory::Profiles,
        SnapshotCategory::Preference => SourceCategory::Preferences,
        SnapshotCategory::Usage => SourceCategory::Usage,
    }
}

/// No write runs before the archive's scope, digest and exact target set have
/// been checked. Targets are owner-issued bundles covering records, references
/// and projections, not client-supplied paths. Persist the receipt before calling.
/// Unsupported capture/restoration must fail before calling this entry point.
///
/// Restore the failed owner too: its error may follow a partial commit. Restore
/// dependents before dependencies and continue recovery after individual errors.
/// No later import category is dispatched after the first commit error. A restore
/// callback must return Ok only after all its bundles and references are restored;
/// it must reject external version changes rather than overwrite them. Neither
/// callback errors nor preimages are safe to expose as raw API messages.
#[allow(clippy::too_many_arguments)]
pub fn apply_with_rollback<E, R>(
    root: &Path,
    receipt: &SafetySnapshotReceipt,
    scope: &SourceScope,
    source: &SourceSnapshot,
    confirmed_source: &SourceSummary,
    selected: &[SourceCategory],
    targets: &[SnapshotTarget],
    commit: impl FnMut(SourceCategory) -> Result<(), E>,
    mut restore: impl FnMut(SourceCategory, &[&OwnerPreimage]) -> Result<(), R>,
) -> Result<RollbackReport<E, R>, RollbackError> {
    let snapshot_scope = SafetySnapshotScope {
        project: scope.project().clone(),
        principal_id: scope.principal_id().into(),
        host_id: scope.host_id().into(),
    };
    let snapshot = open_safety_snapshot(root, receipt, &snapshot_scope, targets)
        .map_err(RollbackError::Snapshot)?;
    let prepared: BTreeSet<_> = targets.iter().map(|target| category(target.category)).collect();
    let selected_set: BTreeSet<_> = selected.iter().copied().collect();
    if prepared != selected_set || selected_set.len() != selected.len() {
        return Err(RollbackError::InvalidSelection);
    }
    let prepared: Vec<_> = prepared.into_iter().collect();
    let apply = apply_selected(scope, source, confirmed_source, selected, &prepared, commit)
        .map_err(RollbackError::Apply)?;
    let mut restored = Vec::new();
    let mut recovery_required = Vec::new();
    if let Some((failed, _)) = &apply.failed {
        for current in std::iter::once(*failed).chain(apply.completed.iter().rev().copied()) {
            let images: Vec<_> = snapshot.preimages().iter()
                .filter(|image| category(image.target.category) == current).collect();
            match restore(current, &images) {
                Ok(()) => restored.push(current),
                Err(error) => recovery_required.push((current, error)),
            }
        }
    }
    Ok(RollbackReport { receipt: receipt.clone(), apply, restored, recovery_required })
}
