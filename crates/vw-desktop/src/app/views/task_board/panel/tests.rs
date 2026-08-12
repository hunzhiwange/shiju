use super::*;
use crate::app::state::DelegateAgentSettingsEntry;
use crate::app::task::{TaskStatus, WorktreePoolSnapshot};
use iced::widget::button;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn snapshot_with_lock(holder: Option<&str>) -> WorktreePoolSnapshot {
    WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 0,
        busy_count: 0,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: holder
            .map(|task_id| vec![("main".to_string(), task_id.to_string())])
            .unwrap_or_default(),
        slots: Vec::new(),
    }
}

#[test]
fn module_is_wired_and_scrollbar_contract_is_narrow() {
    assert!(module_path!().ends_with("panel::tests"));
    assert_eq!(TASK_PANEL_SCROLLBAR_WIDTH, 4.0);
    assert!(TASK_PANEL_SCROLLBAR_GUTTER >= TASK_PANEL_SCROLLBAR_WIDTH);
}

#[test]
fn agent_option_displays_its_user_facing_label() {
    let option =
        TaskBoardAgentOption { key: "reviewer".to_string(), label: "Reviewer".to_string() };

    assert_eq!(option.to_string(), "Reviewer");
}

#[test]
fn task_status_colors_cover_builtin_and_custom_statuses() {
    let cases = [
        (TaskStatus::Pool, (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))),
        (TaskStatus::Pending, (Color::from_rgb8(37, 99, 235), Color::from_rgb8(219, 234, 254))),
        (TaskStatus::Planning, (Color::from_rgb8(79, 70, 229), Color::from_rgb8(224, 231, 255))),
        (TaskStatus::Running, (Color::from_rgb8(147, 51, 234), Color::from_rgb8(243, 232, 255))),
        (TaskStatus::Failed, (Color::from_rgb8(220, 38, 38), Color::from_rgb8(254, 226, 226))),
        (TaskStatus::Paused, (Color::from_rgb8(202, 138, 4), Color::from_rgb8(254, 249, 195))),
        (
            TaskStatus::CodeComplete,
            (Color::from_rgb8(5, 150, 105), Color::from_rgb8(209, 250, 229)),
        ),
        (TaskStatus::CodeReview, (Color::from_rgb8(217, 119, 6), Color::from_rgb8(254, 243, 199))),
        (TaskStatus::PrSubmitted, (Color::from_rgb8(8, 145, 178), Color::from_rgb8(207, 250, 254))),
        (TaskStatus::Completed, (Color::from_rgb8(22, 163, 74), Color::from_rgb8(220, 252, 231))),
        (TaskStatus::Archived, (Color::from_rgb8(100, 116, 139), Color::from_rgb8(241, 245, 249))),
        (
            TaskStatus::Custom("review-ready"),
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246)),
        ),
    ];

    for (status, expected) in cases {
        assert_eq!(task_status_tag_colors(status), expected, "status: {status:?}");
    }
}

#[test]
fn duration_and_running_dots_cover_zero_boundaries_and_cycle() {
    assert_eq!(format_duration_ms(0), "0s");
    assert_eq!(format_duration_ms(999), "0s");
    assert_eq!(format_duration_ms(61_000), "1m 1s");

    assert_eq!(running_dots(0), "·");
    assert_eq!(running_dots(999), "·");
    assert_eq!(running_dots(1_000), "··");
    assert_eq!(running_dots(2_000), "···");
    assert_eq!(running_dots(3_000), "·");
}

#[test]
fn segmented_button_style_distinguishes_theme_selection_and_interaction() {
    for theme in [Theme::Light, Theme::Dark] {
        let active = segmented_button_style(&theme, true, button::Status::Active);
        let inactive = segmented_button_style(&theme, false, button::Status::Disabled);
        let hovered = segmented_button_style(&theme, false, button::Status::Hovered);
        let pressed = segmented_button_style(&theme, true, button::Status::Pressed);

        assert_eq!(active.text_color, theme.palette().primary);
        assert_eq!(inactive.text_color, theme.palette().text);
        assert_eq!(active.border.width, 1.0);
        assert!(active.shadow.blur_radius > inactive.shadow.blur_radius);
        assert!(hovered.background.is_some());
        assert!(pressed.background.is_some());
    }
}

#[test]
fn agent_options_filter_disabled_entries_but_preserve_selection_and_main_fallback() {
    let mut app = test_app();
    app.agents_settings.entries.clear();

    let mut enabled = DelegateAgentSettingsEntry::from_config("enabled", None);
    enabled.label = "Enabled agent".to_string();
    enabled.enabled = true;
    let mut disabled = DelegateAgentSettingsEntry::from_config("disabled", None);
    disabled.label = "Disabled agent".to_string();
    disabled.enabled = false;
    app.agents_settings.entries = vec![enabled, disabled];

    let default_options = build_agent_options(&app, MAIN_AGENT_KEY);
    assert_eq!(default_options[0].key, MAIN_AGENT_KEY);
    assert!(default_options.iter().any(|option| option.key == "enabled"));
    assert!(!default_options.iter().any(|option| option.key == "disabled"));

    let selected_options = build_agent_options(&app, "disabled");
    assert!(
        selected_options
            .iter()
            .any(|option| { option.key == "disabled" && option.label == "Disabled agent" })
    );
}

#[test]
fn agent_options_exclude_retired_builtin_agents() {
    let mut app = test_app();
    app.agents_settings.entries = vec![
        DelegateAgentSettingsEntry::from_config(MAIN_AGENT_KEY, None),
        DelegateAgentSettingsEntry::from_config("build", None),
    ];

    let options = build_agent_options(&app, "build");

    assert!(options.iter().any(|option| option.key == MAIN_AGENT_KEY));
    assert!(!options.iter().any(|option| option.key == "build"));
}

#[test]
fn agent_options_insert_main_when_agent_registry_is_empty() {
    let mut app = test_app();
    app.agents_settings.entries.clear();

    let options = build_agent_options(&app, "missing-agent");

    assert_eq!(options.len(), 1);
    assert_eq!(options[0].key, MAIN_AGENT_KEY);
    assert_eq!(options[0].label, "Main");
}

#[test]
fn contextual_rows_reject_irrelevant_or_blank_task_metadata() {
    let app = test_app();
    let mut task = Task::default();

    assert!(build_failed_reason_row(&task).is_none());
    task.status = TaskStatus::Failed;
    task.last_error = Some("   ".to_string());
    assert!(build_failed_reason_row(&task).is_none());
    task.status = TaskStatus::Paused;
    task.pause_reason = Some("   ".to_string());
    assert!(build_failed_reason_row(&task).is_none());

    assert!(build_selected_worktree_row(&task).is_none());
    task.selected_worktree_path = Some("   ".to_string());
    assert!(build_selected_worktree_row(&task).is_none());

    task.merge_target_branch = Some("main".to_string());
    assert!(build_merge_lock_row(&app, &task).is_none());
}

#[test]
fn contextual_rows_build_failure_pause_worktree_and_merge_lock_variants() {
    let mut app = test_app();
    app.project_path = Some("/repo".to_string());

    let mut task = Task { id: "task-1".to_string(), ..Task::default() };
    task.status = TaskStatus::Failed;
    assert!(build_failed_reason_row(&task).is_some());
    task.last_error = Some("worker exited".to_string());
    assert!(build_failed_reason_row(&task).is_some());

    task.status = TaskStatus::Paused;
    assert!(build_failed_reason_row(&task).is_some());
    task.pause_reason = Some("waiting for review".to_string());
    assert!(build_failed_reason_row(&task).is_some());

    task.selected_worktree_path = Some(" /repo/.worktrees/task-1 ".to_string());
    assert!(build_selected_worktree_row(&task).is_some());
    task.merge_target_branch = Some("main".to_string());

    app.task_board_worktree_snapshot = Some(snapshot_with_lock(None));
    assert!(build_merge_lock_row(&app, &task).is_some());
    app.task_board_worktree_snapshot = Some(snapshot_with_lock(Some("task-1")));
    assert!(build_merge_lock_row(&app, &task).is_some());
    app.task_board_worktree_snapshot = Some(snapshot_with_lock(Some("task-2")));
    assert!(build_merge_lock_row(&app, &task).is_some());
}

#[test]
fn field_and_action_builders_cover_draft_edit_and_bulk_message_paths() {
    let mut app = test_app();

    keep_element(build_priority_field(&app, false));
    keep_element(build_priority_field(&app, true));
    keep_element(build_task_split_mode_selector(&app, false));
    keep_element(build_task_split_mode_selector(&app, true));
    keep_element(build_bulk_task_split_selector(&app));
    keep_element(build_agent_field(&app, false));
    keep_element(build_agent_field(&app, true));
    keep_element(build_bulk_agent_selector(&app));
    keep_element(build_action_buttons(&app, false));
    keep_element(build_action_buttons(&app, true));

    app.task_board_create_submit_success = true;
    app.task_board_edit_submit_success = true;
    keep_element(build_action_buttons(&app, false));
    keep_element(build_action_buttons(&app, true));

    app.task_board_is_import_mode = true;
    keep_element(build_action_buttons(&app, false));
}

#[test]
fn task_panel_builds_create_import_edit_and_logs_layouts() {
    let mut app = test_app();
    keep_element(build_task_panel(&app, false));

    app.task_board_is_import_mode = true;
    app.task_board_import_prompt_collapsed = false;
    keep_element(build_task_panel(&app, false));
    app.task_board_import_prompt_collapsed = true;
    keep_element(build_task_panel(&app, false));

    let mut task = Task { id: "task-panel".to_string(), ..Task::default() };
    task.status = TaskStatus::Failed;
    task.last_error = Some("executor unavailable".to_string());
    task.selected_worktree_path = Some("/repo/.worktrees/task-panel".to_string());
    task.merge_target_branch = Some("main".to_string());
    app.project_path = Some("/repo".to_string());
    app.task_board_worktree_snapshot = Some(snapshot_with_lock(Some("task-panel")));
    app.task_board_viewing_logs = Some(task);
    app.task_board_detail_tab = TaskBoardDetailTab::Basic;
    keep_element(build_task_panel(&app, true));

    app.task_board_detail_tab = TaskBoardDetailTab::Logs;
    keep_element(build_task_panel(&app, true));

    app.task_board_viewing_logs = None;
    app.task_board_detail_tab = TaskBoardDetailTab::Basic;
    keep_element(build_task_panel(&app, true));
}

#[test]
fn supporting_panel_builders_construct_tabs_headers_and_completion_options() {
    let mut app = test_app();

    keep_element(section_block("标题", "说明", text("内容")));
    keep_element(build_detail_tabs(&app));
    std::hint::black_box(build_detail_tab_button(
        "日志",
        TaskBoardDetailTab::Logs,
        TaskBoardDetailTab::Basic,
    ));

    let create_header = build_header_row(
        "新建任务",
        None,
        None,
        build_close_button(TaskBoardMessage::CreateTaskCancelled),
    );
    std::hint::black_box(create_header);

    let mut completed_task =
        Task { id: "completed-task".to_string(), status: TaskStatus::Completed, ..Task::default() };
    completed_task.last_execution_duration_ms = Some(1_500);
    let completed_header = build_header_row(
        "任务详情",
        Some(completed_task.id.as_str()),
        Some(&completed_task),
        build_close_button(TaskBoardMessage::CloseTaskLogs),
    );
    std::hint::black_box(completed_header);

    let running_task =
        Task { id: "running-task".to_string(), status: TaskStatus::Running, ..Task::default() };
    let running_header = build_header_row(
        "任务详情",
        Some(running_task.id.as_str()),
        Some(&running_task),
        build_close_button(TaskBoardMessage::CloseTaskLogs),
    );
    std::hint::black_box(running_header);

    keep_element(build_prompt_field(&app));
    keep_element(build_mode_toggle(&app));
    keep_element(build_clear_prompt_checkbox(&app));
    keep_element(build_close_after_create_checkbox(&app));
    keep_element(build_close_after_edit_checkbox(&app));
    keep_element(build_create_options_row(&app));
    keep_element(build_edit_options_row(&app));

    app.task_board_is_import_mode = true;
    keep_element(build_mode_toggle(&app));
}

#[test]
fn import_field_builds_each_template_format_and_priority_fallback() {
    let mut app = test_app();
    app.task_board_is_import_mode = true;
    app.task_board_draft.priority = "not-a-number".to_string();
    app.task_board_import_prompt_collapsed = false;

    for format in [
        crate::app::task::TaskImportPromptFormat::Json,
        crate::app::task::TaskImportPromptFormat::Csv,
        crate::app::task::TaskImportPromptFormat::Tsv,
    ] {
        app.task_board_import_prompt_format = format;
        keep_element(build_import_field(&app));
    }

    app.task_board_import_prompt_collapsed = true;
    keep_element(build_import_field(&app));
}
