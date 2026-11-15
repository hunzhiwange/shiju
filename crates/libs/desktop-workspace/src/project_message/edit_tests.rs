use super::*;
#[derive(Default)]
struct Fixture { project_editor: Editor,
hovered_recent_project: Option<String>,
project_path: Option<String>,
project_tools_menu_path: Option<String>,
project_worktree_enabled: std::collections::HashMap<String, bool>,
recent_project_remove_confirm_path: Option<String>,
recent_projects: Vec<String>,
recent_projects_edits: Vec<String>,
recent_projects_meta: Vec<RecentProjectMeta>,
session_menu_anchor: Option<iced::Point>,
session_menu_id: Option<String>,
task_board_settings: desktop_task_model::TaskBoardSettings,
work_current_project_menu: Option<String>,
}
fn app() -> Fixture { Fixture::default() }
fn apply(app: &mut Fixture, event: Event) { let _ = handle(&mut State { editor: &mut app.project_editor,
hovered_recent_project: &mut app.hovered_recent_project,
project_path: &mut app.project_path,
project_tools_menu_path: &mut app.project_tools_menu_path,
project_worktree_enabled: &mut app.project_worktree_enabled,
recent_project_remove_confirm_path: &mut app.recent_project_remove_confirm_path,
recent_projects: &mut app.recent_projects,
recent_projects_edits: &mut app.recent_projects_edits,
recent_projects_meta: &mut app.recent_projects_meta,
session_menu_anchor: &mut app.session_menu_anchor,
session_menu_id: &mut app.session_menu_id,
task_board_settings: &mut app.task_board_settings,
work_current_project_menu: &mut app.work_current_project_menu,
}, event); }
fn meta(path: &str, name: &str) -> RecentProjectMeta {
    RecentProjectMeta {
        path: path.to_string(),
        name: name.to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 120,
        icon: Some("rocket".to_string()),
        icon_color: Some("#336699".to_string()),
        worktree_start_command: Some("pnpm dev".to_string()),
    }
}


#[test]
fn tools_menu_toggles_and_closes() {
    let mut app = app();
    let path = "/tmp/project".to_string();

    let _ = apply(
        &mut app,
        Event::ProjectToolsMenuToggled(path.clone()),
    );
    assert_eq!(app.project_tools_menu_path, Some(path.clone()));

    let _ = apply(
        &mut app,
        Event::ProjectToolsMenuToggled(path),
    );
    assert!(app.project_tools_menu_path.is_none());

    app.project_tools_menu_path = Some("/tmp/project".to_string());
    let _ = apply(
        &mut app,
        Event::ProjectToolsMenuClosed,
    );
    assert!(app.project_tools_menu_path.is_none());
}
#[test]
fn edit_opened_prefers_recent_edit_and_loads_meta_fields() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec!["Edited Name".to_string()];
    app.recent_projects_meta = vec![meta(&path, "Meta Name")];
    app.project_worktree_enabled.insert(path.clone(), true);
    app.project_tools_menu_path = Some(path.clone());

    let _ = apply(
        &mut app,
        Event::ProjectEditOpened(path.clone()),
    );

    assert_eq!(app.project_editor.project_edit_path, Some(path));
    assert_eq!(app.project_editor.project_edit_tab, ProjectEditTab::General);
    assert_eq!(app.project_editor.project_edit_name, "Edited Name");
    assert_eq!(app.project_editor.project_edit_icon, "rocket");
    assert_eq!(app.project_editor.project_edit_icon_color, "#336699");
    assert_eq!(app.project_editor.project_edit_start_script, "pnpm dev");
    assert!(app.project_editor.project_edit_worktree_enabled);
    assert_eq!(app.project_editor.project_edit_session_refresh_interval_seconds_input, "120");
    assert!(app.project_tools_menu_path.is_none());
}
#[test]
fn edit_opened_falls_back_to_meta_name_then_path_file_name() {
    let mut app = app();
    let path = "/tmp/project-a".to_string();
    app.recent_projects_meta = vec![meta(&path, "Meta Name")];

    let _ = apply(
        &mut app,
        Event::ProjectEditOpened(path.clone()),
    );
    assert_eq!(app.project_editor.project_edit_name, "Meta Name");

    app.recent_projects_meta.clear();
    let _ = apply(
        &mut app,
        Event::ProjectEditOpened(
            "/tmp/fallback-project".to_string(),
        ),
    );
    assert_eq!(app.project_editor.project_edit_name, "fallback-project");
}
#[test]
fn edit_opened_uses_active_project_settings_and_sanitizes_numeric_fields() {
    let mut app = app();
    let path = "/tmp/active-project".to_string();
    let mut project_meta = meta(&path, "Active Project");
    project_meta.task_board_settings = None;
    project_meta.session_auto_refresh = false;
    project_meta.session_refresh_interval_seconds = 0;
    app.project_path = Some(path.clone());
    app.recent_projects_meta = vec![project_meta];
    app.task_board_settings.max_concurrent = 99;
    app.task_board_settings.refresh_interval_seconds = 0;
    app.task_board_settings.scheduler_tick_interval_seconds = 99;
    app.task_board_settings.auto_promote_tick_interval_seconds = 9999;
    app.task_board_settings.failed_retry_minutes = 0;
    app.task_board_settings.failed_retry_max_attempts = 99;
    app.task_board_settings.running_timeout_minutes = 0;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 1;
    app.project_editor.project_edit_icon_hovered = true;
    app.project_editor.project_edit_icon_color_picker_open = true;
    app.project_editor.project_edit_icon_color_format = desktop_ui_controls::color_picker::ColorFormat::Rgba;

    let _ = apply(
        &mut app,
        Event::ProjectEditOpened(path),
    );

    assert_eq!(app.project_editor.project_edit_max_concurrent_input, "10");
    assert_eq!(app.project_editor.project_edit_task_board_refresh_interval_seconds_input, "1");
    assert_eq!(app.project_editor.project_edit_task_board_scheduler_tick_interval_seconds_input, "60");
    assert_eq!(app.project_editor.project_edit_task_board_auto_promote_tick_interval_seconds_input, "3600");
    assert_eq!(app.project_editor.project_edit_failed_retry_minutes_input, "1");
    assert_eq!(app.project_editor.project_edit_failed_retry_max_attempts_input, "10");
    assert_eq!(app.project_editor.project_edit_running_timeout_minutes_input, "1");
    assert_eq!(app.project_editor.project_edit_pr_submitted_stall_timeout_seconds_input, "5");
    assert!(!app.project_editor.project_edit_session_auto_refresh);
    assert_eq!(app.project_editor.project_edit_session_refresh_interval_seconds_input, "1");
    assert!(!app.project_editor.project_edit_icon_hovered);
    assert!(!app.project_editor.project_edit_icon_color_picker_open);
    assert_eq!(
        app.project_editor.project_edit_icon_color_format,
        desktop_ui_controls::color_picker::ColorFormat::Hex
    );
}
#[test]
fn edit_opened_for_inactive_project_uses_its_saved_task_board_settings() {
    let mut app = app();
    let path = "/tmp/inactive-project".to_string();
    let mut project_meta = meta(&path, "Inactive Project");
    let mut saved_settings = desktop_task_model::TaskBoardSettings::new();
    saved_settings.max_concurrent = 7;
    saved_settings.refresh_interval_seconds = 321;
    saved_settings.scheduler_tick_interval_seconds = 17;
    project_meta.task_board_settings = Some(saved_settings);
    app.project_path = Some("/tmp/active-project".to_string());
    app.task_board_settings.max_concurrent = 2;
    app.task_board_settings.refresh_interval_seconds = 99;
    app.recent_projects_meta = vec![project_meta];

    let _ = apply(
        &mut app,
        Event::ProjectEditOpened(path),
    );

    assert_eq!(app.project_editor.project_edit_max_concurrent_input, "7");
    assert_eq!(app.project_editor.project_edit_task_board_refresh_interval_seconds_input, "321");
    assert_eq!(app.project_editor.project_edit_task_board_scheduler_tick_interval_seconds_input, "17");
}
#[test]
fn edit_opened_without_saved_settings_uses_project_defaults() {
    let mut app = app();
    let path = "/tmp/new-project".to_string();

    let _ = apply(
        &mut app,
        Event::ProjectEditOpened(path),
    );

    let defaults = desktop_task_model::TaskBoardSettings::new();
    assert_eq!(app.project_editor.project_edit_max_concurrent_input, defaults.max_concurrent.to_string());
    assert_eq!(
        app.project_editor.project_edit_task_board_refresh_interval_seconds_input,
        defaults.refresh_interval_seconds.to_string()
    );
    assert_eq!(
        app.project_editor.project_edit_task_board_scheduler_tick_interval_seconds_input,
        defaults.scheduler_tick_interval_seconds.to_string()
    );
    assert_eq!(
        app.project_editor.project_edit_task_board_auto_promote_tick_interval_seconds_input,
        defaults.auto_promote_tick_interval_seconds.to_string()
    );
    assert_eq!(
        app.project_editor.project_edit_session_auto_refresh,
        default_recent_project_session_auto_refresh()
    );
    assert_eq!(
        app.project_editor.project_edit_session_refresh_interval_seconds_input,
        default_recent_project_session_refresh_interval_seconds().to_string()
    );
    assert!(app.project_editor.project_edit_icon.is_empty());
    assert!(app.project_editor.project_edit_icon_color.is_empty());
    assert!(app.project_editor.project_edit_start_script.is_empty());
    assert!(!app.project_editor.project_edit_worktree_enabled);
}
#[test]
fn edit_simple_field_messages_update_state() {
    let mut app = app();

    let _ = apply(
        &mut app,
        Event::ProjectEditTabSelected(
            ProjectEditTab::Scheduling,
        ),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditNameChanged("Name".to_string()),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditIconChanged("icon".to_string()),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditIconHovered(true),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditIconColorChanged(
            "#112233".to_string(),
        ),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditIconColorPickerToggled,
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditStartScriptChanged(
            "cargo run".to_string(),
        ),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditWorktreeToggled(true),
    );

    assert_eq!(app.project_editor.project_edit_tab, ProjectEditTab::Scheduling);
    assert_eq!(app.project_editor.project_edit_name, "Name");
    assert_eq!(app.project_editor.project_edit_icon, "icon");
    assert!(app.project_editor.project_edit_icon_hovered);
    assert_eq!(app.project_editor.project_edit_icon_color, "#112233");
    assert!(app.project_editor.project_edit_icon_color_picker_open);
    assert_eq!(app.project_editor.project_edit_start_script, "cargo run");
    assert!(app.project_editor.project_edit_worktree_enabled);

    let _ = apply(
        &mut app,
        Event::ProjectEditIconColorPickerClosed,
    );
    assert!(!app.project_editor.project_edit_icon_color_picker_open);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。#[test]
fn edit_color_format_preset_editor_action_and_recycle_toggle_update_state() {
    let mut app = app();
    app.project_editor.project_edit_start_script = "first".to_string();
    app.project_editor.project_edit_start_script_editor = iced::widget::text_editor::Content::with_text("first");

    let _ = apply(
        &mut app,
        Event::ProjectEditIconColorPresetSelected(
            "#abcdef".to_string(),
        ),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditIconColorFormatChanged(
            desktop_ui_controls::color_picker::ColorFormat::Rgba,
        ),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditStartScriptEditorAction(
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new(" line".to_string()),
            )),
        ),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditRecycleWorktreeOnTaskFinishToggled(
            true,
        ),
    );

    assert_eq!(app.project_editor.project_edit_icon_color, "#abcdef");
    assert_eq!(
        app.project_editor.project_edit_icon_color_format,
        desktop_ui_controls::color_picker::ColorFormat::Rgba
    );
    assert!(app.project_editor.project_edit_start_script.contains("line"));
    assert!(app.project_editor.project_edit_task_board_settings.recycle_worktree_on_task_finish);
}
#[test]
fn edit_icon_file_picked_ignores_empty_and_accepts_trimmed_path() {
    let mut app = app();
    app.project_editor.project_edit_icon = "old".to_string();

    let _ = apply(
        &mut app,
        Event::ProjectEditIconFilePicked(None),
    );
    assert_eq!(app.project_editor.project_edit_icon, "old");

    let _ = apply(
        &mut app,
        Event::ProjectEditIconFilePicked(Some(
            "   ".to_string(),
        )),
    );
    assert_eq!(app.project_editor.project_edit_icon, "old");

    app.project_editor.project_edit_icon_hovered = true;
    let _ = apply(
        &mut app,
        Event::ProjectEditIconFilePicked(Some(
            " /tmp/icon.png ".to_string(),
        )),
    );
    assert_eq!(app.project_editor.project_edit_icon, "/tmp/icon.png");
    assert!(!app.project_editor.project_edit_icon_hovered);
}
#[test]
fn edit_toggles_and_numeric_changes_clamp_settings() {
    let mut app = app();

    let _ = apply(
        &mut app,
        Event::ProjectEditTaskBoardAutoRefreshToggled(false),
    );
    let _ = apply(
        &mut app,
        Event::ProjectEditSessionAutoRefreshToggled(false),
    );

    assert!(!app.project_editor.project_edit_task_board_auto_refresh);
    assert!(!app.project_editor.project_edit_task_board_settings.auto_refresh);
    assert!(!app.project_editor.project_edit_session_auto_refresh);

    let messages = [
        Event::ProjectEditMaxConcurrentChanged(99),
        Event::ProjectEditSessionRefreshIntervalSecondsChanged(0),
        Event::ProjectEditTaskBoardRefreshIntervalSecondsChanged(9999),
        Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(99),
        Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(0),
        Event::ProjectEditFailedRetryMinutesChanged(9999),
        Event::ProjectEditFailedRetryMaxAttemptsChanged(99),
        Event::ProjectEditRunningTimeoutMinutesChanged(0),
        Event::ProjectEditPrSubmittedStallTimeoutSecondsChanged(1),
    ];
    for message in messages {
        let _ = apply(&mut app, message);
    }

    assert_eq!(app.project_editor.project_edit_task_board_settings.max_concurrent, 10);
    assert_eq!(app.project_editor.project_edit_session_refresh_interval_seconds_input, "1");
    assert_eq!(app.project_editor.project_edit_task_board_settings.refresh_interval_seconds, 3600);
    assert_eq!(app.project_editor.project_edit_task_board_settings.scheduler_tick_interval_seconds, 60);
    assert_eq!(app.project_editor.project_edit_task_board_settings.auto_promote_tick_interval_seconds, 1);
    assert_eq!(app.project_editor.project_edit_task_board_settings.failed_retry_minutes, 1440);
    assert_eq!(app.project_editor.project_edit_task_board_settings.failed_retry_max_attempts, 10);
    assert_eq!(app.project_editor.project_edit_task_board_settings.running_timeout_minutes, 1);
    assert_eq!(app.project_editor.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds, 5);
}
#[test]
fn edit_input_messages_preserve_raw_text() {
    let mut app = app();

    let inputs = [
        Event::ProjectEditMaxConcurrentInputChanged("x".to_string()),
        Event::ProjectEditSessionRefreshIntervalSecondsInputChanged("2".to_string()),
        Event::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged("3".to_string()),
        Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged("4".to_string()),
        Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged("5".to_string()),
        Event::ProjectEditFailedRetryMinutesInputChanged("6".to_string()),
        Event::ProjectEditFailedRetryMaxAttemptsInputChanged("10".to_string()),
        Event::ProjectEditRunningTimeoutMinutesInputChanged("7".to_string()),
        Event::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged("8".to_string()),
    ];
    for message in inputs {
        let _ = apply(&mut app, message);
    }

    assert_eq!(app.project_editor.project_edit_max_concurrent_input, "x");
    assert_eq!(app.project_editor.project_edit_session_refresh_interval_seconds_input, "2");
    assert_eq!(app.project_editor.project_edit_task_board_refresh_interval_seconds_input, "3");
    assert_eq!(app.project_editor.project_edit_task_board_scheduler_tick_interval_seconds_input, "4");
    assert_eq!(app.project_editor.project_edit_task_board_auto_promote_tick_interval_seconds_input, "5");
    assert_eq!(app.project_editor.project_edit_failed_retry_minutes_input, "6");
    assert_eq!(app.project_editor.project_edit_failed_retry_max_attempts_input, "10");
    assert_eq!(app.project_editor.project_edit_running_timeout_minutes_input, "7");
    assert_eq!(app.project_editor.project_edit_pr_submitted_stall_timeout_seconds_input, "8");
}
#[test]
fn edit_cancel_resets_transient_fields() {
    let mut app = app();
    app.project_editor.project_edit_path = Some("/tmp/project".to_string());
    app.project_editor.project_edit_name = "Name".to_string();
    app.project_editor.project_edit_icon = "icon".to_string();
    app.project_editor.project_edit_icon_hovered = true;
    app.project_editor.project_edit_icon_color = "#fff".to_string();
    app.project_editor.project_edit_icon_color_picker_open = true;
    app.project_editor.project_edit_start_script = "run".to_string();
    app.project_editor.project_edit_worktree_enabled = true;
    app.project_editor.project_edit_max_concurrent_input = "10".to_string();
    app.project_editor.project_edit_task_board_auto_refresh = false;

    let _ =
        apply(&mut app, Event::ProjectEditCanceled);

    assert!(app.project_editor.project_edit_path.is_none());
    assert!(app.project_editor.project_edit_name.is_empty());
    assert!(app.project_editor.project_edit_icon.is_empty());
    assert!(!app.project_editor.project_edit_icon_hovered);
    assert!(app.project_editor.project_edit_icon_color.is_empty());
    assert!(!app.project_editor.project_edit_icon_color_picker_open);
    assert!(app.project_editor.project_edit_start_script.is_empty());
    assert!(!app.project_editor.project_edit_worktree_enabled);
    assert!(app.project_editor.project_edit_max_concurrent_input.is_empty());
    assert!(app.project_editor.project_edit_task_board_auto_refresh);
}
#[test]
fn recent_remove_pressed_opens_confirmation_without_removing() {
    let mut app = app();
    let removed = "/tmp/remove".to_string();
    let kept = "/tmp/keep".to_string();
    app.recent_projects = vec![removed.clone(), kept.clone()];
    app.recent_projects_edits = vec!["Remove".to_string(), "Keep".to_string()];
    app.project_tools_menu_path = Some(removed.clone());

    let _ = apply(
        &mut app,
        Event::RecentRemovePressed(removed.clone()),
    );

    assert_eq!(app.recent_projects, vec![removed.clone(), kept]);
    assert_eq!(app.recent_project_remove_confirm_path, Some(removed));
    assert!(app.project_tools_menu_path.is_none());
}
#[test]
fn recent_remove_canceled_clears_confirmation() {
    let mut app = app();
    app.recent_project_remove_confirm_path = Some("/tmp/remove".to_string());

    let _ =
        apply(&mut app, Event::RecentRemoveCanceled);

    assert!(app.recent_project_remove_confirm_path.is_none());
}
#[test]
fn recent_remove_updates_recent_lists_and_clears_related_state() {
    let mut app = app();
    let removed = "/tmp/remove".to_string();
    let kept = "/tmp/keep".to_string();
    app.recent_projects = vec![removed.clone(), kept.clone()];
    app.recent_projects_meta = vec![meta(&kept, "Keep")];
    app.hovered_recent_project = Some(removed.clone());
    app.project_tools_menu_path = Some(removed.clone());
    app.project_editor.project_edit_path = Some(removed.clone());
    app.project_editor.project_edit_name = "Remove".to_string();
    app.recent_project_remove_confirm_path = Some(removed.clone());

    let _ = apply(
        &mut app,
        Event::RecentRemoveConfirmed(removed),
    );

    assert_eq!(app.recent_projects, vec![kept]);
    assert_eq!(app.recent_projects_edits, vec!["Keep".to_string()]);
    assert!(app.hovered_recent_project.is_none());
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.project_editor.project_edit_path.is_none());
    assert!(app.project_editor.project_edit_name.is_empty());
    assert!(app.recent_project_remove_confirm_path.is_none());
}
#[test]
fn recent_remove_rebuilds_edit_names_from_file_names_when_meta_is_missing() {
    let mut app = app();
    let removed = "/tmp/remove".to_string();
    let kept = "/tmp/plain-keep".to_string();
    app.recent_projects = vec![removed.clone(), kept.clone()];
    app.project_tools_menu_path = Some("/tmp/other".to_string());
    app.project_editor.project_edit_path = Some("/tmp/other".to_string());
    app.project_editor.project_edit_name = "Other".to_string();

    let _ = apply(
        &mut app,
        Event::RecentRemoveConfirmed(removed),
    );

    assert_eq!(app.recent_projects, vec![kept]);
    assert_eq!(app.recent_projects_edits, vec!["plain-keep".to_string()]);
    assert_eq!(app.project_tools_menu_path, Some("/tmp/other".to_string()));
    assert_eq!(app.project_editor.project_edit_path, Some("/tmp/other".to_string()));
    assert_eq!(app.project_editor.project_edit_name, "Other");
}
