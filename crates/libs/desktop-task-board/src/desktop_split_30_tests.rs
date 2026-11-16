use desktop_task_model::{Task, TaskSplitMode, TaskStatus};
fn task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(1);
    task.id = id.into();
    task.status = status;
    task
}
#[test]
fn task_metadata_patch_trims_workflow_and_copies_task_fields() {
    let mut task = task("T-metadata", TaskStatus::Pending);
    task.prompt = "First line\nSecond line".to_string();
    task.priority = 42;
    task.order = 7;
    task.model = "provider/model".to_string();
    task.agent = Some("reviewer".to_string());
    task.acp_agent = Some("codex".to_string());
    task.assignee = "Alice".to_string();
    task.description = "description".to_string();
    task.task_split_mode = TaskSplitMode::Enabled;
    task.auto_promote_delay_ms = Some(5_000);

    let patch = crate::bulk::task_metadata_patch_for_gateway(&task, "  dev_lifecycle  ");

    assert_eq!(patch.title.as_deref(), Some("First line"));
    assert_eq!(patch.priority, Some(42));
    assert_eq!(patch.order_no, Some(7));
    assert_eq!(patch.model.as_deref(), Some("provider/model"));
    assert_eq!(patch.agent.as_deref(), Some("reviewer"));
    assert_eq!(patch.acp_agent.as_deref(), Some("codex"));
    assert_eq!(patch.prompt.as_deref(), Some(task.prompt.as_str()));
    assert_eq!(patch.workflow_id.as_deref(), Some("dev_lifecycle"));
    assert!(patch.status_key.is_some());
    assert!(patch.status.is_some());
    assert!(patch.status_role.is_some());
    assert_eq!(
        patch.metadata_merge,
        Some(serde_json::json!({
            "assignee": "Alice",
            "description": "description",
            "subtasks": [],
            "auto_promote_delay_ms": 5_000,
            "task_split_mode": "enabled",
        }))
    );
    assert!(patch.metadata_remove_keys.is_empty());
}

#[test]
fn task_metadata_patch_copies_error_pause_and_worktree_fields() {
    let mut task = task("T-worktree", TaskStatus::PrSubmitted);
    task.last_error = Some("merge conflict".to_string());
    task.pause_reason = Some("manual review".to_string());
    task.merge_source_branch = Some("feature/a".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path = Some("/repo/worktree".to_string());
    task.selected_worktree_slot_id = Some("slot-1".to_string());
    task.selected_worktree_branch = Some("feature/a".to_string());

    let patch = crate::bulk::task_metadata_patch_for_gateway(&task, "workflow");

    assert_eq!(patch.last_error.as_deref(), Some("merge conflict"));
    assert_eq!(patch.pause_reason.as_deref(), Some("manual review"));
    assert_eq!(patch.merge_source_branch.as_deref(), Some("feature/a"));
    assert_eq!(patch.merge_target_branch.as_deref(), Some("main"));
    assert_eq!(patch.selected_worktree_path.as_deref(), Some("/repo/worktree"));
    assert_eq!(patch.selected_worktree_slot_id.as_deref(), Some("slot-1"));
    assert_eq!(patch.selected_worktree_branch.as_deref(), Some("feature/a"));
}
mod settings {
    use crate::helpers::settings::*;
    #[test]
    fn interval_accessors_clamp_lower_middle_and_upper_values() {
        let mut settings = desktop_task_model::TaskBoardSettings::default();

        settings.refresh_interval_seconds = 0;
        settings.scheduler_tick_interval_seconds = 0;
        settings.auto_promote_tick_interval_seconds = 0;
        settings.pr_submitted_stall_timeout_seconds = 0;
        assert_eq!(task_board_refresh_interval_secs(&settings), 1);
        assert_eq!(scheduler_tick_interval_secs(&settings), 1);
        assert_eq!(auto_promote_tick_interval_secs(&settings), 1);
        assert_eq!(pr_submitted_stall_timeout_secs(&settings), 300);

        settings.refresh_interval_seconds = 45;
        settings.scheduler_tick_interval_seconds = 15;
        settings.auto_promote_tick_interval_seconds = 90;
        settings.pr_submitted_stall_timeout_seconds = 600;
        assert_eq!(task_board_refresh_interval_secs(&settings), 45);
        assert_eq!(scheduler_tick_interval_secs(&settings), 15);
        assert_eq!(auto_promote_tick_interval_secs(&settings), 90);
        assert_eq!(pr_submitted_stall_timeout_secs(&settings), 600);

        settings.refresh_interval_seconds = u64::MAX;
        settings.scheduler_tick_interval_seconds = u64::MAX;
        settings.auto_promote_tick_interval_seconds = u64::MAX;
        settings.pr_submitted_stall_timeout_seconds = u32::MAX;
        assert_eq!(task_board_refresh_interval_secs(&settings), 3600);
        assert_eq!(scheduler_tick_interval_secs(&settings), 60);
        assert_eq!(auto_promote_tick_interval_secs(&settings), 3600);
        assert_eq!(pr_submitted_stall_timeout_secs(&settings), 3600);
    }
}

mod logs {
    use desktop_task_model::{Task, TaskLogEntry, TaskStatus};
    use iced::widget::text_editor;
    #[derive(Default)]
    struct LogsFixture {
        task_board_logs_editor: iced::widget::text_editor::Content,
        task_board_logs_auto_scroll: bool,
        task_board_logs_scroll_top_line: f32,
        task_board_logs_scroll_remainder: f32,
        task_board_logs_viewport_height: f32,
        current_line_height: f32,
        desktop_locale: config_ui::DesktopLocale,
        task_board_log_cache: std::collections::HashMap<String, Vec<TaskLogEntry>>,
        task_board_tasks: Vec<Task>,
        task_board_viewing_logs: Option<Task>,
        task_board_logs_context_menu_open: bool,
        task_board_logs_context_menu_pos: Option<(f32, f32)>,
    }
    impl LogsFixture {
        fn state(&mut self) -> crate::helpers::logs::TaskLogsState<'_> {
            crate::helpers::logs::TaskLogsState {
                task_board_logs_editor: &mut self.task_board_logs_editor,
                task_board_logs_auto_scroll: &mut self.task_board_logs_auto_scroll,
                task_board_logs_scroll_top_line: &mut self.task_board_logs_scroll_top_line,
                task_board_logs_scroll_remainder: &mut self.task_board_logs_scroll_remainder,
                task_board_logs_viewport_height: &mut self.task_board_logs_viewport_height,
                current_line_height: &mut self.current_line_height,
                desktop_locale: &mut self.desktop_locale,
                task_board_log_cache: &mut self.task_board_log_cache,
                task_board_tasks: &mut self.task_board_tasks,
                task_board_viewing_logs: &mut self.task_board_viewing_logs,
                task_board_logs_context_menu_open: &mut self.task_board_logs_context_menu_open,
                task_board_logs_context_menu_pos: &mut self.task_board_logs_context_menu_pos,
            }
        }
    }
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

    // 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
    // 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

    // 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
    // 重点确认现有状态与错误反馈不会被意外的部分更新污染。

    #[test]
    fn apply_and_refresh_task_log_cache_merge_active_logs_and_remove_terminal_cache() {
        let mut app = LogsFixture::default();
        app.task_board_log_cache
            .insert("task-1".to_string(), vec![log(1, "one"), log(2, "cached")]);
        let mut active = task("task-1", TaskStatus::Running, vec![log(1, "one")]);

        crate::helpers::logs::apply_cached_logs_to_task(&mut app.state(), &mut active);
        crate::helpers::logs::refresh_task_log_cache(&mut app.state(), &active);

        assert_eq!(active.logs.len(), 2);
        assert_eq!(active.logs[1].message, "cached");
        assert_eq!(app.task_board_log_cache["task-1"].len(), 2);

        active.status = TaskStatus::Completed;
        crate::helpers::logs::refresh_task_log_cache(&mut app.state(), &active);
        assert!(!app.task_board_log_cache.contains_key("task-1"));
    }

    #[test]
    fn refresh_task_log_cache_by_id_updates_only_existing_task() {
        let mut app = LogsFixture::default();
        app.task_board_tasks = vec![task("active", TaskStatus::Planning, vec![log(1, "planning")])];
        app.task_board_log_cache.insert("untouched".to_string(), vec![log(2, "keep")]);

        crate::helpers::logs::refresh_task_log_cache_by_id(&mut app.state(), "active");
        crate::helpers::logs::refresh_task_log_cache_by_id(&mut app.state(), "missing");

        assert_eq!(app.task_board_log_cache["active"][0].message, "planning");
        assert_eq!(app.task_board_log_cache["untouched"][0].message, "keep");
    }

    #[test]
    fn sync_task_log_cache_for_loaded_tasks_drops_stale_and_merges_before_status_cleanup() {
        let mut app = LogsFixture::default();
        app.task_board_tasks = vec![
            task("active", TaskStatus::Running, vec![log(1, "one")]),
            task("done", TaskStatus::Completed, vec![log(3, "done")]),
        ];
        app.task_board_log_cache
            .insert("active".to_string(), vec![log(1, "one"), log(2, "cached active")]);
        app.task_board_log_cache
            .insert("done".to_string(), vec![log(3, "done"), log(4, "cached done")]);
        app.task_board_log_cache.insert("stale".to_string(), vec![log(5, "stale")]);

        crate::helpers::logs::sync_task_log_cache_for_loaded_tasks(&mut app.state());

        assert_eq!(app.task_board_tasks[0].logs.len(), 2);
        assert_eq!(app.task_board_tasks[1].logs.len(), 2);
        assert_eq!(app.task_board_log_cache["active"].len(), 2);
        assert!(!app.task_board_log_cache.contains_key("done"));
        assert!(!app.task_board_log_cache.contains_key("stale"));
    }

    #[test]
    fn set_viewing_logs_applies_cache_and_syncs_editor_or_clears_it() {
        let mut app = LogsFixture::default();
        app.task_board_log_cache.insert("viewed".to_string(), vec![log(1_000, "cached")]);
        let viewed = task("viewed", TaskStatus::Running, Vec::new());

        crate::helpers::logs::set_viewing_logs(&mut app.state(), Some(viewed));

        assert_eq!(app.task_board_viewing_logs.as_ref().expect("viewed task").logs.len(), 1);
        assert_eq!(app.task_board_logs_editor.text(), "[00:00:01] cached");

        crate::helpers::logs::set_viewing_logs(&mut app.state(), None);
        assert!(app.task_board_viewing_logs.is_none());
        assert!(app.task_board_logs_editor.text().is_empty());
    }

    // 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
    // 完成、失败与过期响应均按当前场景的既定契约处理。
    #[test]
    fn sync_viewing_logs_updates_viewed_task_and_other_task_cache() {
        let mut app = LogsFixture::default();
        app.task_board_tasks = vec![
            task("viewed", TaskStatus::Running, vec![log(1_000, "updated viewed")]),
            task("other", TaskStatus::Planning, vec![log(2_000, "updated other")]),
        ];
        app.task_board_viewing_logs = Some(task("viewed", TaskStatus::Running, Vec::new()));

        crate::helpers::logs::sync_viewing_logs(&mut app.state(), "viewed");
        crate::helpers::logs::sync_viewing_logs(&mut app.state(), "other");

        assert_eq!(
            app.task_board_viewing_logs.as_ref().expect("viewed task").logs[0].message,
            "updated viewed"
        );
        assert!(app.task_board_logs_editor.text().contains("updated viewed"));
        assert_eq!(app.task_board_log_cache["other"][0].message, "updated other");
    }

    #[test]
    fn close_logs_context_menu_resets_open_state_and_position() {
        let mut app = LogsFixture::default();
        app.task_board_logs_context_menu_open = true;
        app.task_board_logs_context_menu_pos = Some((12.0, 34.0));

        crate::helpers::logs::close_logs_context_menu(&mut app.state());

        assert!(!app.task_board_logs_context_menu_open);
        assert_eq!(app.task_board_logs_context_menu_pos, None);
    }

    #[test]
    fn task_logs_max_scroll_top_line_uses_visible_lines_and_safe_minimums() {
        let mut app = LogsFixture::default();
        app.task_board_logs_editor = text_editor::Content::with_text("one\ntwo\nthree\nfour\nfive");
        app.task_board_logs_viewport_height = 40.0;
        app.current_line_height = 20.0;

        assert_eq!(
            crate::helpers::logs::max_scroll_top_line(
                app.task_board_logs_viewport_height,
                app.current_line_height,
                app.task_board_logs_editor.line_count()
            ),
            3.0
        );

        app.task_board_logs_viewport_height = 0.0;
        app.current_line_height = 0.0;
        assert_eq!(
            crate::helpers::logs::max_scroll_top_line(
                app.task_board_logs_viewport_height,
                app.current_line_height,
                app.task_board_logs_editor.line_count()
            ),
            4.0
        );
    }

    #[test]
    fn apply_task_logs_scroll_lines_clamps_bounds_and_tracks_bottom() {
        let mut app = LogsFixture::default();
        app.task_board_logs_editor = text_editor::Content::with_text("one\ntwo\nthree\nfour\nfive");
        app.task_board_logs_viewport_height = 40.0;
        app.current_line_height = 20.0;
        app.task_board_logs_scroll_top_line = 1.0;

        crate::helpers::logs::apply_task_logs_scroll_lines(&mut app.state(), 99);
        assert_eq!(app.task_board_logs_scroll_top_line, 3.0);
        assert!(app.task_board_logs_auto_scroll);

        crate::helpers::logs::apply_task_logs_scroll_lines(&mut app.state(), -99);
        assert_eq!(app.task_board_logs_scroll_top_line, 0.0);
        assert!(!app.task_board_logs_auto_scroll);
    }

    #[test]
    fn restore_and_bottom_scroll_round_and_clamp_target_line() {
        let mut app = LogsFixture::default();
        app.task_board_logs_editor = text_editor::Content::with_text("one\ntwo\nthree\nfour\nfive");
        app.task_board_logs_viewport_height = 40.0;
        app.current_line_height = 20.0;

        crate::helpers::logs::restore_task_logs_scroll(&mut app.state(), 1.6);
        assert_eq!(app.task_board_logs_scroll_top_line, 2.0);
        assert!(!app.task_board_logs_auto_scroll);

        crate::helpers::logs::scroll_task_logs_to_bottom(&mut app.state());
        assert_eq!(app.task_board_logs_scroll_top_line, 3.0);
        assert!(app.task_board_logs_auto_scroll);
    }

    // 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
    // 完成、失败与过期响应均按当前场景的既定契约处理。
    #[test]
    fn sync_task_logs_editor_replaces_text_resets_remainder_and_restores_position() {
        let mut app = LogsFixture::default();
        app.task_board_logs_editor = text_editor::Content::with_text("old");
        app.task_board_logs_auto_scroll = false;
        app.task_board_logs_scroll_top_line = 1.0;
        app.task_board_logs_scroll_remainder = 0.75;
        app.task_board_logs_viewport_height = 20.0;
        app.current_line_height = 20.0;
        let task = task("sync", TaskStatus::Running, vec![log(1_000, "one\ntwo\nthree")]);

        crate::helpers::logs::sync_task_logs_editor(&mut app.state(), Some(&task));

        assert_eq!(
            app.task_board_logs_editor.text(),
            "[00:00:01] one\n[00:00:01] two\n[00:00:01] three"
        );
        assert_eq!(app.task_board_logs_scroll_remainder, 0.0);
        assert_eq!(app.task_board_logs_scroll_top_line, 1.0);
        assert!(!app.task_board_logs_auto_scroll);
    }
}

#[test]
fn task_metadata_patch_uses_id_and_default_workflow_for_empty_inputs() {
    let task = task("T-fallback-title", TaskStatus::Pool);

    let patch = crate::bulk::task_metadata_patch_for_gateway(&task, "   ");

    assert_eq!(patch.title.as_deref(), Some("T-fallback-title"));
    assert_eq!(patch.workflow_id.as_deref(), Some(desktop_task_model::DEFAULT_TASK_WORKFLOW_ID));
}
