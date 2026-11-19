use super::*;

#[test]
fn codex_file_failure_keeps_later_imports_and_reports_partial_scan() {
    use crate::scan_jobs::{ScanCounts, ScanFailure, ScanPhase};

    let owner = ScanOwner {
        scope: vw_api_transport::ProjectScope::new(
            vw_api_transport::TenantScope::default(),
            vw_api_transport::local_project_id(),
        )
        .unwrap(),
        principal: "desktop-local-user".into(),
        target_host: vw_api_transport::local_host_id(),
    };
    let mut schedule = ScanSchedule::new(ScanJobs::default(), owner);
    schedule.set_source(ScanAdapter::Codex, UsageSourceSettings::default());
    let job = schedule.manual_sync(Instant::now()).unwrap();
    let mut steps = [
        ScanStep::FileFailed(ScanFailure::SourceUnavailable),
        ScanStep::FileFailed(ScanFailure::SourceUnavailable),
        ScanStep::Committed(ScanCounts { imported: 2, files_scanned: 1, ..Default::default() }),
        ScanStep::End,
    ].into_iter();
    let result = schedule.execute(&job, |adapter| {
        assert_eq!(adapter, ScanAdapter::Codex);
        steps.next().unwrap()
    }).unwrap();

    assert_eq!(result.counts.imported, 2);
    assert_eq!(result.errors, [(ScanAdapter::Codex, ScanFailure::SourceUnavailable)]);
    assert_eq!(result.phase, ScanPhase::Failed);
    assert!(schedule.last_complete_scan().unwrap().is_none());
    assert!(steps.next().is_none());
}

#[test]
fn periodic_scans_follow_the_selected_interval_with_or_without_the_page() {
    let now = Instant::now();
    let owner = ScanOwner {
        scope: vw_api_transport::ProjectScope::new(
            vw_api_transport::TenantScope::default(),
            vw_api_transport::local_project_id(),
        )
        .unwrap(),
        principal: "desktop-local-user".into(),
        target_host: vw_api_transport::local_host_id(),
    };
    let mut schedule = ScanSchedule::new(ScanJobs::default(), owner);
    schedule.set_source(ScanAdapter::Codex, UsageSourceSettings::default());

    let startup = schedule.tick(now, true).unwrap().unwrap();
    schedule.execute(&startup, |_| ScanStep::End).unwrap();
    schedule.set_page_active(true);
    let opened = now + Duration::from_secs(1);
    let page = schedule.tick(opened, true).unwrap().unwrap();
    schedule.execute(&page, |_| ScanStep::End).unwrap();
    let second = Duration::from_secs(1);
    let day = Duration::from_secs(86_400);
    assert!(schedule.tick(opened + Duration::from_secs(600), true).unwrap().is_none());
    assert!(schedule.tick(opened + day - second, true).unwrap().is_none());
    let periodic = schedule.tick(opened + day, true).unwrap().unwrap();
    schedule.execute(&periodic, |_| ScanStep::End).unwrap();

    let hour = Duration::from_secs(3_600);
    schedule.set_preferences(Some(UsageScanPreferences { enabled: true, refresh_seconds: 3_600 }));
    let next = opened + day + hour;
    assert!(schedule.tick(next - second, true).unwrap().is_none());
    let periodic = schedule.tick(next, true).unwrap().unwrap();
    schedule.execute(&periodic, |_| ScanStep::End).unwrap();

    schedule.set_page_active(false);
    let background = next + hour;
    assert!(schedule.tick(background - second, true).unwrap().is_none());
    let job = schedule.tick(background, true).unwrap().unwrap();
    schedule.execute(&job, |_| ScanStep::End).unwrap();

    schedule.set_preferences(Some(UsageScanPreferences { enabled: false, refresh_seconds: 3_600 }));
    let deadline = background + hour;
    assert!(schedule.tick(deadline, true).unwrap().is_none());
    let manual = schedule.manual_sync(deadline).unwrap();
    assert_eq!(schedule.tick(deadline, true).unwrap().unwrap().task_id, manual.task_id);
}
