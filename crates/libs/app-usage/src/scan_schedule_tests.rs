use super::*;

#[test]
fn startup_and_page_scans_follow_the_selected_interval() {
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
    let page = schedule.tick(now + Duration::from_secs(1), true).unwrap().unwrap();
    schedule.execute(&page, |_| ScanStep::End).unwrap();
    assert!(schedule.tick(now + Duration::from_secs(60), true).unwrap().is_none());
    let periodic = schedule.tick(now + Duration::from_secs(61), true).unwrap().unwrap();
    schedule.execute(&periodic, |_| ScanStep::End).unwrap();

    schedule.set_preferences(Some(UsageScanPreferences { enabled: true, refresh_seconds: 5 }));
    assert!(schedule.tick(now + Duration::from_secs(65), true).unwrap().is_none());
    let next = now + Duration::from_secs(66);
    let periodic = schedule.tick(next, true).unwrap().unwrap();
    schedule.execute(&periodic, |_| ScanStep::End).unwrap();

    schedule.set_page_active(false);
    let background = next + BACKGROUND_SCAN_INTERVAL;
    assert!(schedule.tick(background - Duration::from_secs(1), true).unwrap().is_none());
    let job = schedule.tick(background, true).unwrap().unwrap();
    schedule.execute(&job, |_| ScanStep::End).unwrap();

    schedule.set_preferences(Some(UsageScanPreferences { enabled: false, refresh_seconds: 5 }));
    let deadline = background + BACKGROUND_SCAN_INTERVAL;
    assert!(schedule.tick(deadline, true).unwrap().is_none());
    let manual = schedule.manual_sync(deadline).unwrap();
    assert_eq!(schedule.tick(deadline, true).unwrap().unwrap().task_id, manual.task_id);
}
