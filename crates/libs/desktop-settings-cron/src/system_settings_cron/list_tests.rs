use super::*;
use super::super::helpers_tests::job;
#[test]
fn wb_schedule_modal_expired_card_ignores_retained_next_run() {
    let mut job = job("ended");
    job.ends_at = Some("2026-01-02T00:00:00Z".into());
    job.next_run = "2099-01-01T00:00:00Z".into();
    let now = "2026-01-02T00:00:00Z".parse().unwrap();
    assert!(job_has_ended(&job, now));
    job.ends_at = Some("2026-01-03T00:00:00Z".into());
    assert!(!job_has_ended(&job, now));
}
