use super::*;

fn snapshot(phase: &str) -> UsageJobSnapshot {
    serde_json::from_value(serde_json::json!({"task_id":"fixture-job", "phase":phase})).unwrap()
}

#[test]
fn usage_replica_06_terminal_feedback_distinguishes_failure_and_cancellation() {
    for phase in ["Completed", "Failed", "Cancelled"] {
        let job = snapshot(phase);
        assert!(terminal(&job));
        assert_eq!(job_failed(&job), phase != "Completed");
    }
    assert!(!terminal(&snapshot("Running")));
}

#[test]
fn usage_replica_06_close_keeps_job_and_clears_rebuild_confirmation() {
    let mut state = State { open: true, confirm_rebuild: true, job: Some(snapshot("Running")), ..State::default() };
    let _task = update(&mut state, None, Err("unused transport".into()), Event::Close);
    assert!(!state.open);
    assert!(!state.confirm_rebuild);
    assert_eq!(state.job.as_ref().unwrap().task_id, "fixture-job");
}

#[test]
fn usage_replica_06_unscoped_rebuild_never_starts_work() {
    let mut state = State::default();
    let _task = update(&mut state, None, Err("unused transport".into()), Event::Run(Action::Rebuild));
    assert!(!state.pending);
    assert!(state.job.is_none());
}

#[test]
fn usage_replica_06_navigation_closes_sheet_without_cancelling_job() {
    let mut state = State { open: true, job: Some(snapshot("Running")), ..State::default() };
    let _task = update(&mut state, None, Err("unused transport".into()), Event::Navigate(Navigation::Routing));
    assert!(!state.open);
    assert!(matches!(take_navigation(&mut state), Some(Navigation::Routing)));
    assert!(take_navigation(&mut state).is_none());
    assert!(!terminal(state.job.as_ref().unwrap()));
}

#[test]
fn usage_replica_06_maintenance_terminal_status_reports_failure() {
    let job: UsageJobSnapshot = serde_json::from_value(serde_json::json!({
        "task_id":"fixture-maintenance", "phase":"Terminal", "status":"failed"
    })).unwrap();
    assert!(terminal(&job));
    assert!(job_failed(&job));
}
