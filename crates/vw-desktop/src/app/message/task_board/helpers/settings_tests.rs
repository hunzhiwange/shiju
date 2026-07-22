use super::*;

fn assert_deadline_in_window(deadline_ms: u64, start_ms: u64, end_ms: u64, interval_secs: u64) {
    let interval_ms = interval_secs.saturating_mul(1000);

    assert!(deadline_ms >= start_ms.saturating_add(interval_ms));
    assert!(deadline_ms <= end_ms.saturating_add(interval_ms));
}

#[test]
fn save_settings_without_project_leaves_recent_projects_unchanged() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.recent_projects_meta.clear();

    save_settings(&mut app);

    assert!(app.recent_projects_meta.is_empty());
}

#[test]
fn save_settings_updates_existing_project_meta() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project-a".to_string());
    app.task_board_settings.max_concurrent = 7;
    app.recent_projects_meta = vec![crate::app::state::RecentProjectMeta {
        path: "/tmp/project-a".to_string(),
        name: "Project A".to_string(),
        task_board_settings: None,
        session_auto_refresh: false,
        session_refresh_interval_seconds: 10,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }];

    save_settings(&mut app);

    let saved =
        app.recent_projects_meta[0].task_board_settings.as_ref().expect("settings should be saved");
    assert_eq!(saved.max_concurrent, 7);
    assert_eq!(app.recent_projects_meta[0].name, "Project A");
}

#[test]
fn save_settings_creates_project_meta_for_new_project_path() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/tmp/project-b".to_string());
    app.recent_projects_meta.clear();
    app.task_board_settings.max_concurrent = 3;

    save_settings(&mut app);

    assert_eq!(app.recent_projects_meta.len(), 1);
    let meta = &app.recent_projects_meta[0];
    assert_eq!(meta.path, "/tmp/project-b");
    assert_eq!(meta.name, "project-b");
    assert_eq!(
        meta.task_board_settings.as_ref().expect("settings should be saved").max_concurrent,
        3
    );
}

#[test]
fn next_deadline_saturates_when_interval_overflows_milliseconds() {
    assert_eq!(next_deadline_ms(u64::MAX), u64::MAX);
}

#[test]
fn interval_accessors_clamp_lower_middle_and_upper_values() {
    let (mut app, _task) = crate::app::App::new();

    app.task_board_settings.refresh_interval_seconds = 0;
    app.task_board_settings.scheduler_tick_interval_seconds = 0;
    app.task_board_settings.auto_promote_tick_interval_seconds = 0;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 0;
    assert_eq!(task_board_refresh_interval_secs(&app), 1);
    assert_eq!(scheduler_tick_interval_secs(&app), 1);
    assert_eq!(auto_promote_tick_interval_secs(&app), 1);
    assert_eq!(pr_submitted_stall_timeout_secs(&app), 300);

    app.task_board_settings.refresh_interval_seconds = 45;
    app.task_board_settings.scheduler_tick_interval_seconds = 15;
    app.task_board_settings.auto_promote_tick_interval_seconds = 90;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 600;
    assert_eq!(task_board_refresh_interval_secs(&app), 45);
    assert_eq!(scheduler_tick_interval_secs(&app), 15);
    assert_eq!(auto_promote_tick_interval_secs(&app), 90);
    assert_eq!(pr_submitted_stall_timeout_secs(&app), 600);

    app.task_board_settings.refresh_interval_seconds = u64::MAX;
    app.task_board_settings.scheduler_tick_interval_seconds = u64::MAX;
    app.task_board_settings.auto_promote_tick_interval_seconds = u64::MAX;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = u32::MAX;
    assert_eq!(task_board_refresh_interval_secs(&app), 3600);
    assert_eq!(scheduler_tick_interval_secs(&app), 60);
    assert_eq!(auto_promote_tick_interval_secs(&app), 3600);
    assert_eq!(pr_submitted_stall_timeout_secs(&app), 3600);
}

#[test]
fn sanitized_settings_raise_lower_bounds_and_replace_unknown_workflow() {
    let mut settings = TaskBoardSettings::new();
    settings.max_concurrent = 0;
    settings.refresh_interval_seconds = 0;
    settings.scheduler_tick_interval_seconds = 0;
    settings.auto_promote_tick_interval_seconds = 0;
    settings.failed_retry_minutes = 0;
    settings.failed_retry_max_attempts = 0;
    settings.running_timeout_minutes = 0;
    settings.pr_submitted_stall_timeout_seconds = 0;
    settings.workflow_id = "unknown-workflow".to_string();

    let sanitized = sanitized_task_board_settings(settings);

    assert_eq!(sanitized.max_concurrent, 1);
    assert_eq!(sanitized.refresh_interval_seconds, 1);
    assert_eq!(sanitized.scheduler_tick_interval_seconds, 1);
    assert_eq!(sanitized.auto_promote_tick_interval_seconds, 1);
    assert_eq!(sanitized.failed_retry_minutes, 1);
    assert_eq!(sanitized.failed_retry_max_attempts, 0);
    assert_eq!(sanitized.running_timeout_minutes, 1);
    assert_eq!(sanitized.pr_submitted_stall_timeout_seconds, 5);
    assert_eq!(sanitized.workflow_id, crate::app::task::DEFAULT_TASK_WORKFLOW_ID);
}

#[test]
fn sanitized_settings_clamp_upper_bounds_and_preserve_valid_workflow() {
    let mut settings = TaskBoardSettings::new();
    settings.max_concurrent = u32::MAX;
    settings.refresh_interval_seconds = u64::MAX;
    settings.scheduler_tick_interval_seconds = u64::MAX;
    settings.auto_promote_tick_interval_seconds = u64::MAX;
    settings.failed_retry_minutes = u32::MAX;
    settings.failed_retry_max_attempts = u32::MAX;
    settings.running_timeout_minutes = u32::MAX;
    settings.pr_submitted_stall_timeout_seconds = u32::MAX;
    settings.workflow_id = crate::app::task::DEFAULT_TASK_WORKFLOW_ID.to_string();

    let sanitized = sanitized_task_board_settings(settings);

    assert_eq!(sanitized.max_concurrent, 10);
    assert_eq!(sanitized.refresh_interval_seconds, 3600);
    assert_eq!(sanitized.scheduler_tick_interval_seconds, 60);
    assert_eq!(sanitized.auto_promote_tick_interval_seconds, 3600);
    assert_eq!(sanitized.failed_retry_minutes, 1440);
    assert_eq!(sanitized.failed_retry_max_attempts, 10);
    assert_eq!(sanitized.running_timeout_minutes, 1440);
    assert_eq!(sanitized.pr_submitted_stall_timeout_seconds, 3600);
    assert_eq!(sanitized.workflow_id, crate::app::task::DEFAULT_TASK_WORKFLOW_ID);
}

#[test]
fn scheduler_deadline_is_preserved_while_still_in_the_future() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_next_scheduler_tick_at_ms = u64::MAX;

    let _scheduled = schedule_scheduler_tick_with_deadline(&mut app);

    assert_eq!(app.task_board_next_scheduler_tick_at_ms, u64::MAX);
}

#[test]
fn scheduler_deadline_is_refreshed_after_expiry() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.scheduler_tick_interval_seconds = 2;
    app.task_board_next_scheduler_tick_at_ms = 0;
    let start_ms = now_ms();

    let _scheduled = schedule_scheduler_tick_with_deadline(&mut app);

    let end_ms = now_ms();
    assert_deadline_in_window(app.task_board_next_scheduler_tick_at_ms, start_ms, end_ms, 2);
}

#[test]
fn fixed_interval_schedulers_refresh_their_deadlines() {
    let (mut app, _task) = crate::app::App::new();

    let review_start_ms = now_ms();
    let _review_task = schedule_auto_review_tick_with_deadline(&mut app);
    let review_end_ms = now_ms();
    assert_deadline_in_window(
        app.task_board_next_auto_review_tick_at_ms,
        review_start_ms,
        review_end_ms,
        TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS,
    );

    let merge_start_ms = now_ms();
    let _merge_task = schedule_auto_merge_tick_with_deadline(&mut app);
    let merge_end_ms = now_ms();
    assert_deadline_in_window(
        app.task_board_next_auto_merge_tick_at_ms,
        merge_start_ms,
        merge_end_ms,
        TASK_AUTO_MERGE_TICK_INTERVAL_SECS,
    );
}

#[test]
fn auto_promote_scheduler_uses_sanitized_interval_for_deadline() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.auto_promote_tick_interval_seconds = 0;
    let start_ms = now_ms();

    let _scheduled = schedule_auto_promote_tick_with_deadline(&mut app);

    let end_ms = now_ms();
    assert_deadline_in_window(app.task_board_next_auto_promote_tick_at_ms, start_ms, end_ms, 1);
}

#[test]
fn standalone_schedulers_produce_deferred_actions() {
    let (app, _task) = crate::app::App::new();

    for task in
        [schedule_scheduler_tick(&app), schedule_auto_review_tick(), schedule_auto_merge_tick()]
    {
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
