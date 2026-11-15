use desktop_task_model::{Task, TaskLogEntry, TaskStatus};
fn log(timestamp_ms: u64, message: impl Into<String>) -> TaskLogEntry {
    TaskLogEntry {
        localized_message: None,
        timestamp_ms,
        status_from: None,
        status_to: None,
        message: message.into(),
    }
}

fn task(id: &str, status: TaskStatus, logs: Vec<TaskLogEntry>) -> Task {
    Task { id: id.to_string(), status, logs, ..Task::default() }
}

#[test]
fn format_task_log_timestamp_truncates_milliseconds_and_wraps_days() {
    assert_eq!(super::format_task_log_timestamp(0), "[00:00:00] ");
    assert_eq!(super::format_task_log_timestamp(3_723_999), "[01:02:03] ");
    assert_eq!(super::format_task_log_timestamp(90_061_000), "[01:01:01] ");
}

#[test]
fn task_logs_viewer_text_trims_lines_and_omits_blank_content() {
    let task = task(
        "viewer",
        TaskStatus::Running,
        vec![log(1_000, " first \n\n second "), log(2_000, "  \n third\n")],
    );

    assert_eq!(
        super::task_logs_viewer_text(&task),
        "[00:00:01] first\n[00:00:01] second\n[00:00:02] third"
    );
}

#[test]
fn task_logs_viewer_text_keeps_only_latest_visible_entries() {
    let logs = (0..super::TASK_BOARD_LOG_VIEWER_MAX_VISIBLE + 2)
        .map(|index| log(index as u64 * 1_000, format!("entry-{index}")))
        .collect();
    let task = task("bounded", TaskStatus::Running, logs);

    let text = super::task_logs_viewer_text(&task);

    assert_eq!(text.lines().count(), super::TASK_BOARD_LOG_VIEWER_MAX_VISIBLE);
    assert!(text.starts_with("[00:00:02] entry-2"));
    assert!(text.ends_with("[00:06:41] entry-401"));
    assert!(!text.contains("entry-1\n"));
}

#[test]
fn task_logs_should_stay_in_memory_matches_active_lifecycle_states() {
    for status in
        [TaskStatus::Planning, TaskStatus::Running, TaskStatus::CodeReview, TaskStatus::PrSubmitted]
    {
        assert!(super::task_logs_should_stay_in_memory(status));
    }

    for status in [
        TaskStatus::Pool,
        TaskStatus::Pending,
        TaskStatus::Failed,
        TaskStatus::Paused,
        TaskStatus::CodeComplete,
        TaskStatus::Completed,
        TaskStatus::Archived,
        TaskStatus::Custom("custom"),
    ] {
        assert!(!super::task_logs_should_stay_in_memory(status));
    }
}

#[test]
fn merge_task_logs_with_cache_appends_only_tail_after_matching_prefix() {
    let current = vec![log(1, "one"), log(2, "two")];
    let cached = vec![log(1, "one"), log(2, "two"), log(3, "three")];

    let merged = super::merge_task_logs_with_cache(&current, Some(&cached));

    assert_eq!(
        merged.iter().map(|entry| entry.message.as_str()).collect::<Vec<_>>(),
        ["one", "two", "three"]
    );
}

#[test]
fn merge_task_logs_with_cache_preserves_divergent_and_absent_cache_entries() {
    let current = vec![log(1, "current")];
    let divergent = vec![log(1, "cached"), log(2, "tail")];

    let merged = super::merge_task_logs_with_cache(&current, Some(&divergent));
    let unchanged = super::merge_task_logs_with_cache(&current, None);

    assert_eq!(
        merged.iter().map(|entry| entry.message.as_str()).collect::<Vec<_>>(),
        ["current", "cached", "tail"]
    );
    assert_eq!(unchanged.len(), 1);
    assert_eq!(unchanged[0].message, "current");
}
