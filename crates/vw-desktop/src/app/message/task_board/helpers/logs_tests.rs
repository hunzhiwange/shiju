use crate::app::task::{Task, TaskLogEntry, TaskStatus};
use iced::widget::text_editor;

fn log(timestamp_ms: u64, message: impl Into<String>) -> TaskLogEntry {
    TaskLogEntry { timestamp_ms, status_from: None, status_to: None, message: message.into() }
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

#[test]
fn apply_and_refresh_task_log_cache_merge_active_logs_and_remove_terminal_cache() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_log_cache.insert("task-1".to_string(), vec![log(1, "one"), log(2, "cached")]);
    let mut active = task("task-1", TaskStatus::Running, vec![log(1, "one")]);

    super::apply_cached_logs_to_task(&mut app, &mut active);
    super::refresh_task_log_cache(&mut app, &active);

    assert_eq!(active.logs.len(), 2);
    assert_eq!(active.logs[1].message, "cached");
    assert_eq!(app.task_board_log_cache["task-1"].len(), 2);

    active.status = TaskStatus::Completed;
    super::refresh_task_log_cache(&mut app, &active);
    assert!(!app.task_board_log_cache.contains_key("task-1"));
}

#[test]
fn refresh_task_log_cache_by_id_updates_only_existing_task() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_tasks = vec![task("active", TaskStatus::Planning, vec![log(1, "planning")])];
    app.task_board_log_cache.insert("untouched".to_string(), vec![log(2, "keep")]);

    super::refresh_task_log_cache_by_id(&mut app, "active");
    super::refresh_task_log_cache_by_id(&mut app, "missing");

    assert_eq!(app.task_board_log_cache["active"][0].message, "planning");
    assert_eq!(app.task_board_log_cache["untouched"][0].message, "keep");
}

#[test]
fn sync_task_log_cache_for_loaded_tasks_drops_stale_and_merges_before_status_cleanup() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_tasks = vec![
        task("active", TaskStatus::Running, vec![log(1, "one")]),
        task("done", TaskStatus::Completed, vec![log(3, "done")]),
    ];
    app.task_board_log_cache
        .insert("active".to_string(), vec![log(1, "one"), log(2, "cached active")]);
    app.task_board_log_cache
        .insert("done".to_string(), vec![log(3, "done"), log(4, "cached done")]);
    app.task_board_log_cache.insert("stale".to_string(), vec![log(5, "stale")]);

    super::sync_task_log_cache_for_loaded_tasks(&mut app);

    assert_eq!(app.task_board_tasks[0].logs.len(), 2);
    assert_eq!(app.task_board_tasks[1].logs.len(), 2);
    assert_eq!(app.task_board_log_cache["active"].len(), 2);
    assert!(!app.task_board_log_cache.contains_key("done"));
    assert!(!app.task_board_log_cache.contains_key("stale"));
}

#[test]
fn set_viewing_logs_applies_cache_and_syncs_editor_or_clears_it() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_log_cache.insert("viewed".to_string(), vec![log(1_000, "cached")]);
    let viewed = task("viewed", TaskStatus::Running, Vec::new());

    super::set_viewing_logs(&mut app, Some(viewed));

    assert_eq!(app.task_board_viewing_logs.as_ref().expect("viewed task").logs.len(), 1);
    assert_eq!(app.task_board_logs_editor.text(), "[00:00:01] cached");

    super::set_viewing_logs(&mut app, None);
    assert!(app.task_board_viewing_logs.is_none());
    assert!(app.task_board_logs_editor.text().is_empty());
}

#[test]
fn sync_viewing_logs_updates_viewed_task_and_other_task_cache() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_tasks = vec![
        task("viewed", TaskStatus::Running, vec![log(1_000, "updated viewed")]),
        task("other", TaskStatus::Planning, vec![log(2_000, "updated other")]),
    ];
    app.task_board_viewing_logs = Some(task("viewed", TaskStatus::Running, Vec::new()));

    super::sync_viewing_logs(&mut app, "viewed");
    super::sync_viewing_logs(&mut app, "other");

    assert_eq!(
        app.task_board_viewing_logs.as_ref().expect("viewed task").logs[0].message,
        "updated viewed"
    );
    assert!(app.task_board_logs_editor.text().contains("updated viewed"));
    assert_eq!(app.task_board_log_cache["other"][0].message, "updated other");
}

#[test]
fn close_logs_context_menu_resets_open_state_and_position() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_logs_context_menu_open = true;
    app.task_board_logs_context_menu_pos = Some((12.0, 34.0));

    super::close_logs_context_menu(&mut app);

    assert!(!app.task_board_logs_context_menu_open);
    assert_eq!(app.task_board_logs_context_menu_pos, None);
}

#[test]
fn task_logs_max_scroll_top_line_uses_visible_lines_and_safe_minimums() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_logs_editor = text_editor::Content::with_text("one\ntwo\nthree\nfour\nfive");
    app.task_board_logs_viewport_height = 40.0;
    app.current_line_height = 20.0;

    assert_eq!(super::task_logs_max_scroll_top_line(&app), 3.0);

    app.task_board_logs_viewport_height = 0.0;
    app.current_line_height = 0.0;
    assert_eq!(super::task_logs_max_scroll_top_line(&app), 4.0);
}

#[test]
fn apply_task_logs_scroll_lines_clamps_bounds_and_tracks_bottom() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_logs_editor = text_editor::Content::with_text("one\ntwo\nthree\nfour\nfive");
    app.task_board_logs_viewport_height = 40.0;
    app.current_line_height = 20.0;
    app.task_board_logs_scroll_top_line = 1.0;

    super::apply_task_logs_scroll_lines(&mut app, 99);
    assert_eq!(app.task_board_logs_scroll_top_line, 3.0);
    assert!(app.task_board_logs_auto_scroll);

    super::apply_task_logs_scroll_lines(&mut app, -99);
    assert_eq!(app.task_board_logs_scroll_top_line, 0.0);
    assert!(!app.task_board_logs_auto_scroll);
}

#[test]
fn restore_and_bottom_scroll_round_and_clamp_target_line() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_logs_editor = text_editor::Content::with_text("one\ntwo\nthree\nfour\nfive");
    app.task_board_logs_viewport_height = 40.0;
    app.current_line_height = 20.0;

    super::restore_task_logs_scroll(&mut app, 1.6);
    assert_eq!(app.task_board_logs_scroll_top_line, 2.0);
    assert!(!app.task_board_logs_auto_scroll);

    super::scroll_task_logs_to_bottom(&mut app);
    assert_eq!(app.task_board_logs_scroll_top_line, 3.0);
    assert!(app.task_board_logs_auto_scroll);
}

#[test]
fn sync_task_logs_editor_replaces_text_resets_remainder_and_restores_position() {
    let (mut app, _command) = crate::app::App::new();
    app.task_board_logs_editor = text_editor::Content::with_text("old");
    app.task_board_logs_auto_scroll = false;
    app.task_board_logs_scroll_top_line = 1.0;
    app.task_board_logs_scroll_remainder = 0.75;
    app.task_board_logs_viewport_height = 20.0;
    app.current_line_height = 20.0;
    let task = task("sync", TaskStatus::Running, vec![log(1_000, "one\ntwo\nthree")]);

    super::sync_task_logs_editor(&mut app, Some(&task));

    assert_eq!(
        app.task_board_logs_editor.text(),
        "[00:00:01] one\n[00:00:01] two\n[00:00:01] three"
    );
    assert_eq!(app.task_board_logs_scroll_remainder, 0.0);
    assert_eq!(app.task_board_logs_scroll_top_line, 1.0);
    assert!(!app.task_board_logs_auto_scroll);
}
