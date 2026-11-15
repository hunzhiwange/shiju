use super::*;
#[test]
fn next_deadline_saturates_when_interval_overflows_milliseconds() {
    assert_eq!(next_deadline_ms(u64::MAX), u64::MAX);
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
    assert_eq!(sanitized.workflow_id, desktop_task_model::DEFAULT_TASK_WORKFLOW_ID);
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
    settings.workflow_id = desktop_task_model::DEFAULT_TASK_WORKFLOW_ID.to_string();

    let sanitized = sanitized_task_board_settings(settings);

    assert_eq!(sanitized.max_concurrent, 10);
    assert_eq!(sanitized.refresh_interval_seconds, 3600);
    assert_eq!(sanitized.scheduler_tick_interval_seconds, 60);
    assert_eq!(sanitized.auto_promote_tick_interval_seconds, 3600);
    assert_eq!(sanitized.failed_retry_minutes, 1440);
    assert_eq!(sanitized.failed_retry_max_attempts, 10);
    assert_eq!(sanitized.running_timeout_minutes, 1440);
    assert_eq!(sanitized.pr_submitted_stall_timeout_seconds, 3600);
    assert_eq!(sanitized.workflow_id, desktop_task_model::DEFAULT_TASK_WORKFLOW_ID);
}
