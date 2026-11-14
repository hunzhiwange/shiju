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

/// Publication lease retained through the settings commit and rollback. Both
/// single-resource callers and the multi-resource receive owner use this path.
pub trait SnapshotSkillUpdate: Send + Sync {
    fn publish(&mut self) -> anyhow::Result<()>;
    fn rollback(&mut self) -> anyhow::Result<()>;
    fn finish(self: Box<Self>);
}
impl SnapshotSkillUpdate for SkillUpdate {
    fn publish(&mut self) -> anyhow::Result<()> { Ok(SkillUpdate::publish(self)?) }
    fn rollback(&mut self) -> anyhow::Result<()> { Ok(SkillUpdate::rollback(self)?) }
    fn finish(self: Box<Self>) { SkillUpdate::finish(*self); }
}
impl SnapshotSkillUpdate for vw_agent_apps::device_sync::skill_update::BatchSkillUpdate {
    fn publish(&mut self) -> anyhow::Result<()> { Ok(Self::publish(self)?) }
    fn rollback(&mut self) -> anyhow::Result<()> { Ok(Self::rollback(self)?) }
    fn finish(self: Box<Self>) { Self::finish(*self); }
}

/// Failure always retains the durable receipt. A failed rollback also retains
/// owner guards and staging trees until explicit recovery or drop; the skill
/// owner preserves displaced trees on disk if drop cannot safely restore them.
/// Deliberately excludes driver errors, paths and preimages from diagnostics.
pub struct ApplyFailure {
    pub receipt: SafetySnapshotReceipt,
    pub phase: ApplyPhase,
    pub rollback_failed: bool,
    pending_skills: Vec<Box<dyn SnapshotSkillUpdate>>,
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

fn rollback(skills: &mut [Box<dyn SnapshotSkillUpdate>]) -> bool {
    let mut failed = false;
    for skill in skills.iter_mut().rev() {
        if skill.rollback().is_err() {
            failed = true;
        }
    }
    failed
}

/// Caller holds the tenant/host sync lock through awaited projections. Selected
/// resource guards cover commit; release them before owners reacquire them for
/// projection. `skills` are locally authorized, hash-checked
/// staging updates from the skill owner, never objects decoded from remote JSON.
/// Capture must include complete configuration and skill preimages for exactly
/// `targets`, under those same guards (including absence and native projections).
/// The existing snapshot owner verifies capture and durable readback before any
/// publication. Configuration preimages are rechecked by its database transaction.
///
/// The outer error means no publication was attempted. The inner error retains
/// recovery material even if rollback succeeded; it must never be reported as a
/// successful synchronization. No cancellation/drop boundary occurs during commit. After commit, await
/// projection completion inline; do not select cancellation against this future.
#[allow(clippy::too_many_arguments)]
pub async fn apply_snapshot<S: SnapshotSkillUpdate + 'static>(
    database: &GlobalDatabase,
    _sync_lock: &tokio::sync::OwnedMutexGuard<()>,
    scope: SnapshotScope<'_>,
    preview: SnapshotImport,
    skills: Vec<S>,
    recovery_root: &Path,
    targets: &[SnapshotTarget],
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
    projections: Vec<super::projection::ProjectionJob<'_>>,
) -> Result<Result<super::projection::AppliedSnapshot, ApplyFailure>, SafetySnapshotError> {
    let mut skills: Vec<Box<dyn SnapshotSkillUpdate>> = skills.into_iter().map(|skill| Box::new(skill) as Box<dyn SnapshotSkillUpdate>).collect();
    let safety_scope = SafetySnapshotScope {
        project: scope.project.clone(),
        principal_id: scope.principal.into(),
        host_id: scope.host.into(),
    };
    let receipt = create_safety_snapshot(recovery_root, &safety_scope, targets, capture)?;
    let publication = super::change_events::without_echo(scope.project.tenant_scope(), || {
        for skill in &mut skills {
            if skill.publish().is_err() {
                let rollback_failed = rollback(&mut skills);
                return Err(ApplyFailure {
                    receipt,
                    phase: ApplyPhase::Skills,
                    rollback_failed,
                    pending_skills: skills,
                });
            }
        }
        match apply_shared_snapshot(database, scope, preview) {
            Ok(changed) => {
                // Disarm filesystem rollback only after the database owner commits.
                for skill in skills {
                    skill.finish();
                }
                Ok((changed, receipt))
            }
            Err(_) => {
                let rollback_failed = rollback(&mut skills);
                Err(ApplyFailure {
                    receipt,
                    phase: ApplyPhase::Configuration,
                    rollback_failed,
                    pending_skills: skills,
                })
            }
        }
    });
    match publication {
        Ok((changed, receipt)) => {
            let warnings = super::projection::refresh(projections).await;
            Ok(Ok(super::projection::AppliedSnapshot {
                receipt,
                changed_records: changed,
                warnings,
            }))
        }
        Err(failure) => Ok(Err(failure)),
    }
}
