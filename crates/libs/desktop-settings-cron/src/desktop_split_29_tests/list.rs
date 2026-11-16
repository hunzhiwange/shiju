use super::*;

#[test]
fn jobs_loaded_reconciles_selection_and_preserves_jobs_on_error() {
    let mut app = app();
    app.jobs_loading = true;
    app.selected_job_ids =
        vec!["a".to_string(), "missing".to_string(), "b".to_string()];
    app.expanded_job_ids = vec!["missing".to_string(), "b".to_string()];
    app.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::CronJobsLoaded(Ok(vec![job("a"), job("b")])));

    assert!(!app.jobs_loading);
    assert_eq!(app.selected_job_ids, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(app.expanded_job_ids, vec!["b".to_string()]);
    assert_eq!(app.jobs.len(), 2);
    assert_eq!(app.save_error, None);

    let _ = update(&mut app, SettingsMessage::CronJobsLoaded(Err("offline".to_string())));

    assert_eq!(app.jobs.len(), 2);
    assert_eq!(app.save_error.as_deref(), Some("offline"));
}

#[test]
fn job_selection_is_idempotent_and_select_all_can_be_cleared() {
    let mut app = app();
    app.jobs = vec![job("a"), job("b")];

    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("a".to_string(), true));
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("a".to_string(), true));
    assert_eq!(app.selected_job_ids, vec!["a".to_string()]);

    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("a".to_string(), false));
    assert!(app.selected_job_ids.is_empty());

    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    assert_eq!(app.selected_job_ids, vec!["a".to_string(), "b".to_string()]);
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(false));
    assert!(app.selected_job_ids.is_empty());
}

#[test]
fn job_details_are_collapsed_by_default_and_toggle_idempotently() {
    let mut app = app();
    assert!(app.expanded_job_ids.is_empty());

    let _ = update(&mut app, SettingsMessage::CronJobDetailsToggled("a".to_string(), true));
    let _ = update(&mut app, SettingsMessage::CronJobDetailsToggled("a".to_string(), true));
    assert_eq!(app.expanded_job_ids, vec!["a".to_string()]);

    let _ = update(&mut app, SettingsMessage::CronJobDetailsToggled("a".to_string(), false));
    assert!(app.expanded_job_ids.is_empty());
}

#[test]
fn cron_list_batch_rejects_stale_selection_without_request() {
    let mut app = app();
    app.selected_job_ids = vec!["missing".into()];
    for message in [
        SettingsMessage::CronSelectedJobsEnable,
        SettingsMessage::CronSelectedJobsDisable,
        SettingsMessage::CronSelectedJobsDelete,
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task).is_none());
        assert!(app.save_error.is_some());
    }
}

#[test]
fn cron_list_filter_counts_enabled_and_paused() {
    let mut app = app();
    assert_eq!(app.list_filter, CronListFilter::All);
    assert!(!app.batch_mode);
    app.jobs = cron_list_jobs();
    for (filter, count) in
        [(CronListFilter::All, 3), (CronListFilter::Enabled, 2), (CronListFilter::Paused, 1)]
    {
        let _ = update(&mut app, SettingsMessage::CronListFilterChanged(filter));
        assert_eq!(app.job_count(filter), count);
        assert_eq!(app.job_count(CronListFilter::All), 3);
        assert_eq!(app.visible_jobs().count(), count);
    }
}

#[test]
fn cron_list_select_all_targets_visible_jobs() {
    let mut app = app();
    app.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronBatchModeToggled(true));
    for (filter, expected) in [
        (CronListFilter::Enabled, vec!["enabled-2", "enabled-1"]),
        (CronListFilter::Paused, vec!["paused"]),
        (CronListFilter::All, vec!["enabled-2", "paused", "enabled-1"]),
    ] {
        let _ = update(&mut app, SettingsMessage::CronListFilterChanged(filter));
        let task = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
        assert!(iced_runtime::task::into_stream(task).is_none());
        assert_eq!(app.selected_job_ids, expected);
        let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(false));
        assert!(app.selected_job_ids.is_empty());
    }
}

#[test]
fn cron_list_filter_change_clears_selection() {
    let mut app = app();
    app.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    let _ = update(&mut app, SettingsMessage::CronListFilterChanged(CronListFilter::Paused));
    assert!(app.selected_job_ids.is_empty());
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("enabled-2".into(), true));
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("missing".into(), true));
    assert!(app.selected_job_ids.is_empty());
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("paused".into(), true));
    assert_eq!(app.selected_job_ids, vec!["paused"]);
}

#[test]
fn cron_list_batch_exit_clears_selection() {
    let mut app = app();
    app.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronBatchModeToggled(true));
    assert!(app.batch_mode);
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    let task = update(&mut app, SettingsMessage::CronBatchModeToggled(false));
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(!app.batch_mode);
    assert!(app.selected_job_ids.is_empty());
}

#[test]
fn cron_list_reload_prunes_hidden_selection() {
    let mut app = app();
    app.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronListFilterChanged(CronListFilter::Enabled));
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    let mut changed = job("enabled-2");
    changed.enabled = false;
    let task = update(&mut app, SettingsMessage::CronJobsLoaded(Ok(vec![changed, job("new")])));
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(app.selected_job_ids.is_empty());
    assert_eq!(
        app.visible_jobs().map(|job| job.id.as_str()).collect::<Vec<_>>(),
        vec!["new"]
    );
}

#[test]
fn cron_list_batch_limits_requests_to_visible_valid_selection() {
    let mut app = app();
    app.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronListFilterChanged(CronListFilter::Enabled));
    for message in [
        SettingsMessage::CronSelectedJobsEnable,
        SettingsMessage::CronSelectedJobsDisable,
        SettingsMessage::CronSelectedJobsDelete,
    ] {
        app.selected_job_ids = vec!["paused".into(), "missing".into()];
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task).is_none());
        assert!(app.selected_job_ids.is_empty());
    }
    for message in [
        SettingsMessage::CronSelectedJobsEnable,
        SettingsMessage::CronSelectedJobsDisable,
        SettingsMessage::CronSelectedJobsDelete,
    ] {
        app.selected_job_ids =
            vec!["paused".into(), "enabled-1".into(), "missing".into()];
        let task = update(&mut app, message);
        // Drop the unpolled stream: verify the request boundary without contacting a gateway.
        assert!(iced_runtime::task::into_stream(task).is_some());
        assert_eq!(app.selected_job_ids, vec!["enabled-1"]);
        assert!(app.save_error.is_none());
    }
}
