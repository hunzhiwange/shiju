//! Cancellation is cooperative until the persisted commit boundary.
use super::jobs::{SavedSkillJob, SkillJob, SkillJobPhase};

/// Terminal requests are idempotent; committing jobs retain their real outcome.
pub fn request_cancel(job: &mut SavedSkillJob) -> SkillJob {
    if matches!(job.snapshot.phase, SkillJobPhase::Started | SkillJobPhase::Staged) {
        job.cancel_requested = true;
    }
    job.snapshot.clone()
}
