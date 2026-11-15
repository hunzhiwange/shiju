use super::jobs::{SavedSkillJob, SkillJobPhase};

#[test]
fn cp12_jobs_success() {
    let mut job = SavedSkillJob::new("old-process".into());
    let id = job.snapshot.task_id.clone();
    job.snapshot.phase = SkillJobPhase::Committing;
    job.interrupt_previous_execution("new-process");
    assert_eq!(job.snapshot.task_id, id);
    assert_eq!(job.snapshot.phase, SkillJobPhase::Interrupted);
    assert!(job.snapshot.requires_reconciliation);
}

#[test]
fn cp12_jobs_rejects_without_side_effects() {
    let mut job = SavedSkillJob::new("process".into());
    let original = job.snapshot.clone();
    job.interrupt_previous_execution("process");
    assert_eq!(job.snapshot, original);
    job.snapshot.phase = SkillJobPhase::Completed;
    job.snapshot.installed_directory = Some("skill".into());
    let completed = job.snapshot.clone();
    job.interrupt_previous_execution("next-process");
    assert_eq!(job.snapshot, completed);
}
