//! Apply under the executor's sync lock; no remote paths or SQL are accepted.
use std::path::Path;

use vw_agent_apps::{
    data_management::safety_snapshot::{
        OwnerPreimage, SafetySnapshotError, SafetySnapshotReceipt, SafetySnapshotScope,
        SnapshotTarget, create_safety_snapshot,
    },
    skills::update_apply::SkillUpdate,
};

use crate::{
    agent_apps::{
        shared_snapshot::SnapshotScope,
        shared_snapshot_import::{SnapshotImport, apply_shared_snapshot},
    },
    global_database::GlobalDatabase,
};

/// Failure always retains the durable receipt. A failed rollback also retains
/// owner guards and staging trees until explicit recovery or drop; the skill
/// owner preserves displaced trees on disk if drop cannot safely restore them.
/// Deliberately excludes driver errors, paths and preimages from diagnostics.
pub struct ApplyFailure {
    pub receipt: SafetySnapshotReceipt,
    pub phase: ApplyPhase,
    pub rollback_failed: bool,
    pending_skills: Vec<SkillUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyPhase {
    Skills,
    Configuration,
}

impl std::fmt::Debug for ApplyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplyFailure")
            .field("receipt", &self.receipt)
            .field("phase", &self.phase)
            .field("rollback_failed", &self.rollback_failed)
            .finish()
    }
}
impl std::fmt::Display for ApplyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "device sync apply failed during {:?}; rollback_failed={}; recovery={}",
            self.phase,
            self.rollback_failed,
            self.receipt.id()
        )
    }
}
impl std::error::Error for ApplyFailure {}
impl ApplyFailure {
    /// Retry under the same authorized owner boundary. Never discard external edits.
    pub fn retry_rollback(&mut self) -> bool {
        self.rollback_failed = rollback(&mut self.pending_skills);
        !self.rollback_failed
    }
}

fn rollback(skills: &mut [SkillUpdate]) -> bool {
    let mut failed = false;
    for skill in skills.iter_mut().rev() {
        if skill.rollback().is_err() {
            failed = true;
        }
    }
    failed
}

/// Caller holds the tenant/host sync lock and all selected resource guards for
/// this entire synchronous call. `skills` are locally authorized, hash-checked
/// staging updates from the skill owner, never objects decoded from remote JSON.
/// Capture must include complete configuration and skill preimages for exactly
/// `targets`, under those same guards (including absence and native projections).
/// The existing snapshot owner verifies capture and durable readback before any
/// publication. Configuration preimages are rechecked by its database transaction.
///
/// The outer error means no publication was attempted. The inner error retains
/// recovery material even if rollback succeeded; it must never be reported as a
/// successful synchronization. No cancellation/drop boundary occurs during commit.
#[allow(clippy::too_many_arguments)]
pub fn apply_snapshot(
    database: &GlobalDatabase,
    scope: SnapshotScope<'_>,
    preview: SnapshotImport,
    mut skills: Vec<SkillUpdate>,
    recovery_root: &Path,
    targets: &[SnapshotTarget],
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
) -> Result<Result<(SafetySnapshotReceipt, usize), ApplyFailure>, SafetySnapshotError> {
    let safety_scope = SafetySnapshotScope {
        project: scope.project.clone(),
        principal_id: scope.principal.into(),
        host_id: scope.host.into(),
    };
    let receipt = create_safety_snapshot(recovery_root, &safety_scope, targets, capture)?;
    for skill in &mut skills {
        if skill.publish().is_err() {
            let rollback_failed = rollback(&mut skills);
            return Ok(Err(ApplyFailure {
                receipt,
                phase: ApplyPhase::Skills,
                rollback_failed,
                pending_skills: skills,
            }));
        }
    }
    match apply_shared_snapshot(database, scope, preview) {
        Ok(changed) => {
            // Disarm filesystem rollback only after the database owner commits.
            for skill in skills {
                skill.finish();
            }
            Ok(Ok((receipt, changed)))
        }
        Err(_) => {
            let rollback_failed = rollback(&mut skills);
            Ok(Err(ApplyFailure {
                receipt,
                phase: ApplyPhase::Configuration,
                rollback_failed,
                pending_skills: skills,
            }))
        }
    }
}
