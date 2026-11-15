use super::{job_cancel::request_cancel, jobs::{SavedSkillJob, SkillJobPhase}};

#[test]
fn cp12_job_cancel_success() {
    let mut job = SavedSkillJob::new("executor".into());
    let original = job.snapshot.clone();
    assert_eq!(request_cancel(&mut job), original);
    assert!(job.cancel_requested);
    // The worker must clean its staging directory before reporting interruption.
    assert_eq!(job.snapshot.phase, SkillJobPhase::Started);
    assert_eq!(request_cancel(&mut job), original);
}

#[test]
fn cp12_job_cancel_rejects_without_side_effects() {
    for phase in [SkillJobPhase::Committing, SkillJobPhase::Completed,
        SkillJobPhase::Failed, SkillJobPhase::Interrupted] {
        let mut job = SavedSkillJob::new("executor".into());
        job.snapshot.phase = phase;
        if phase == SkillJobPhase::Completed {
            job.snapshot.installed_directory = Some("skill".into());
        }
        let original = job.snapshot.clone();
        assert_eq!(request_cancel(&mut job), original);
        assert!(!job.cancel_requested);
    }
}
