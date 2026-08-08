#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("edit_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn meta(path: &str, name: &str) -> crate::app::state::RecentProjectMeta {
    crate::app::state::RecentProjectMeta {
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

fn custom_status(key: &'static str) -> crate::app::task::TaskStatus {
    crate::app::task::TaskStatus::parse_key(key).expect("custom status key should parse")
}

fn owned_status(
    key: &'static str,
    label: &'static str,
    role: crate::app::task::StatusRole,
    next: Option<crate::app::task::TaskStatus>,
) -> crate::app::task::OwnedStatusDefinition {
    crate::app::task::OwnedStatusDefinition {
        status: custom_status(key),
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        is_terminal: next.is_none(),
        auto_execution_candidate: !matches!(
            role,
            crate::app::task::StatusRole::Done | crate::app::task::StatusRole::Archived
        ),
        occupies_execution_slot: matches!(
            role,
            crate::app::task::StatusRole::Executing | crate::app::task::StatusRole::Planning
        ),
        is_scheduling_backlog: matches!(
            role,
            crate::app::task::StatusRole::Backlog
                | crate::app::task::StatusRole::Queued
                | crate::app::task::StatusRole::Planning
        ),
        next,
        transitions: Vec::new(),
    }
}

fn custom_workflow(id: &str) -> crate::app::task::OwnedTaskWorkflow {
    let done = custom_status("project_custom_done");
    crate::app::task::OwnedTaskWorkflow {
        id: id.to_string(),
        label: "Project Custom Workflow".to_string(),
        statuses: vec![
            owned_status(
                "project_custom_queued",
                "Queued",
                crate::app::task::StatusRole::Queued,
                Some(done),
            ),
            owned_status("project_custom_done", "Done", crate::app::task::StatusRole::Done, None),
        ],
    }
}

struct TempHome {
    _dir: tempfile::TempDir,
    previous_home: Option<std::ffi::OsString>,
}

impl TempHome {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp home should be created");
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", dir.path()) };
        Self { _dir: dir, previous_home }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match &self.previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

struct RuntimeWorkflowGuard {
    _cache_lock: std::sync::MutexGuard<'static, ()>,
    _home: TempHome,
}

impl RuntimeWorkflowGuard {
    fn with_workflows(workflows: Vec<crate::app::task::OwnedTaskWorkflow>) -> Self {
        let cache_lock = crate::app::task::runtime_task_lifecycle_cache_test_lock();
        let home = TempHome::new();
        crate::app::task::replace_runtime_task_lifecycle_workflows_for_tests(workflows)
            .expect("runtime workflows should be isolated");
        Self { _cache_lock: cache_lock, _home: home }
    }
}

impl Drop for RuntimeWorkflowGuard {
    fn drop(&mut self) {
        let _ = crate::app::task::reset_runtime_task_lifecycle_cache();
    }
}

#[test]
fn tools_menu_toggles_and_closes() {
    let mut app = app();
    let path = "/tmp/project".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectToolsMenuToggled(path.clone()),
    );
    assert_eq!(app.project_tools_menu_path, Some(path.clone()));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectToolsMenuToggled(path),
    );
    assert!(app.project_tools_menu_path.is_none());

    app.project_tools_menu_path = Some("/tmp/project".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectToolsMenuClosed,
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

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditOpened(path.clone()),
    );

    assert_eq!(app.project_edit_path, Some(path));
    assert_eq!(app.project_edit_tab, crate::app::state::ProjectEditTab::General);
    assert_eq!(app.project_edit_name, "Edited Name");
    assert_eq!(app.project_edit_icon, "rocket");
    assert_eq!(app.project_edit_icon_color, "#336699");
    assert_eq!(app.project_edit_start_script, "pnpm dev");
    assert!(app.project_edit_worktree_enabled);
    assert_eq!(app.project_edit_session_refresh_interval_seconds_input, "120");
    assert!(app.project_tools_menu_path.is_none());
}

#[test]
fn edit_opened_falls_back_to_meta_name_then_path_file_name() {
    let mut app = app();
    let path = "/tmp/project-a".to_string();
    app.recent_projects_meta = vec![meta(&path, "Meta Name")];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditOpened(path.clone()),
    );
    assert_eq!(app.project_edit_name, "Meta Name");

    app.recent_projects_meta.clear();
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditOpened(
            "/tmp/fallback-project".to_string(),
        ),
    );
    assert_eq!(app.project_edit_name, "fallback-project");
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
    app.project_edit_icon_hovered = true;
    app.project_edit_icon_color_picker_open = true;
    app.project_edit_icon_color_format = crate::app::components::color_picker::ColorFormat::Rgba;

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditOpened(path),
    );

    assert_eq!(app.project_edit_max_concurrent_input, "10");
    assert_eq!(app.project_edit_task_board_refresh_interval_seconds_input, "1");
    assert_eq!(app.project_edit_task_board_scheduler_tick_interval_seconds_input, "60");
    assert_eq!(app.project_edit_task_board_auto_promote_tick_interval_seconds_input, "3600");
    assert_eq!(app.project_edit_failed_retry_minutes_input, "1");
    assert_eq!(app.project_edit_failed_retry_max_attempts_input, "10");
    assert_eq!(app.project_edit_running_timeout_minutes_input, "1");
    assert_eq!(app.project_edit_pr_submitted_stall_timeout_seconds_input, "5");
    assert!(!app.project_edit_session_auto_refresh);
    assert_eq!(app.project_edit_session_refresh_interval_seconds_input, "1");
    assert!(!app.project_edit_icon_hovered);
    assert!(!app.project_edit_icon_color_picker_open);
    assert_eq!(
        app.project_edit_icon_color_format,
        crate::app::components::color_picker::ColorFormat::Hex
    );
}

#[test]
fn edit_opened_for_inactive_project_uses_its_saved_task_board_settings() {
    let mut app = app();
    let path = "/tmp/inactive-project".to_string();
    let mut project_meta = meta(&path, "Inactive Project");
    let mut saved_settings = crate::app::task::TaskBoardSettings::new();
    saved_settings.max_concurrent = 7;
    saved_settings.refresh_interval_seconds = 321;
    saved_settings.scheduler_tick_interval_seconds = 17;
    project_meta.task_board_settings = Some(saved_settings);
    app.project_path = Some("/tmp/active-project".to_string());
    app.task_board_settings.max_concurrent = 2;
    app.task_board_settings.refresh_interval_seconds = 99;
    app.recent_projects_meta = vec![project_meta];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditOpened(path),
    );

    assert_eq!(app.project_edit_max_concurrent_input, "7");
    assert_eq!(app.project_edit_task_board_refresh_interval_seconds_input, "321");
    assert_eq!(app.project_edit_task_board_scheduler_tick_interval_seconds_input, "17");
}

#[test]
fn edit_opened_without_saved_settings_uses_project_defaults() {
    let mut app = app();
    let path = "/tmp/new-project".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditOpened(path),
    );

    let defaults = crate::app::task::TaskBoardSettings::new();
    assert_eq!(app.project_edit_max_concurrent_input, defaults.max_concurrent.to_string());
    assert_eq!(
        app.project_edit_task_board_refresh_interval_seconds_input,
        defaults.refresh_interval_seconds.to_string()
    );
    assert_eq!(
        app.project_edit_task_board_scheduler_tick_interval_seconds_input,
        defaults.scheduler_tick_interval_seconds.to_string()
    );
    assert_eq!(
        app.project_edit_task_board_auto_promote_tick_interval_seconds_input,
        defaults.auto_promote_tick_interval_seconds.to_string()
    );
    assert_eq!(
        app.project_edit_session_auto_refresh,
        crate::app::state::default_recent_project_session_auto_refresh()
    );
    assert_eq!(
        app.project_edit_session_refresh_interval_seconds_input,
        crate::app::state::default_recent_project_session_refresh_interval_seconds().to_string()
    );
    assert!(app.project_edit_icon.is_empty());
    assert!(app.project_edit_icon_color.is_empty());
    assert!(app.project_edit_start_script.is_empty());
    assert!(!app.project_edit_worktree_enabled);
}

#[test]
fn edit_simple_field_messages_update_state() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditTabSelected(
            crate::app::state::ProjectEditTab::Scheduling,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditNameChanged("Name".to_string()),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconChanged("icon".to_string()),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconHovered(true),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconColorChanged(
            "#112233".to_string(),
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconColorPickerToggled,
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditStartScriptChanged(
            "cargo run".to_string(),
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditWorktreeToggled(true),
    );

    assert_eq!(app.project_edit_tab, crate::app::state::ProjectEditTab::Scheduling);
    assert_eq!(app.project_edit_name, "Name");
    assert_eq!(app.project_edit_icon, "icon");
    assert!(app.project_edit_icon_hovered);
    assert_eq!(app.project_edit_icon_color, "#112233");
    assert!(app.project_edit_icon_color_picker_open);
    assert_eq!(app.project_edit_start_script, "cargo run");
    assert!(app.project_edit_worktree_enabled);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconColorPickerClosed,
    );
    assert!(!app.project_edit_icon_color_picker_open);
}

#[test]
fn edit_color_format_preset_editor_action_and_recycle_toggle_update_state() {
    let mut app = app();
    app.project_edit_start_script = "first".to_string();
    app.project_edit_start_script_editor = iced::widget::text_editor::Content::with_text("first");

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconColorPresetSelected(
            "#abcdef".to_string(),
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconColorFormatChanged(
            crate::app::components::color_picker::ColorFormat::Rgba,
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditStartScriptEditorAction(
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new(" line".to_string()),
            )),
        ),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditRecycleWorktreeOnTaskFinishToggled(
            true,
        ),
    );

    assert_eq!(app.project_edit_icon_color, "#abcdef");
    assert_eq!(
        app.project_edit_icon_color_format,
        crate::app::components::color_picker::ColorFormat::Rgba
    );
    assert!(app.project_edit_start_script.contains("line"));
    assert!(app.project_edit_task_board_settings.recycle_worktree_on_task_finish);
}

#[test]
fn project_accepts_custom_registered_workflow() {
    let workflow_id = "project_custom_workflow";
    let _guard = RuntimeWorkflowGuard::with_workflows(vec![custom_workflow(workflow_id)]);
    let mut app = app();
    let path = "/tmp/project-custom-workflow".to_string();
    app.project_path = Some(path.clone());
    app.recent_projects = vec![path.clone()];
    app.project_edit_path = Some(path.clone());
    app.project_edit_name = "Project".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditWorkflowIdChanged(
            workflow_id.to_string(),
        ),
    );

    assert_eq!(app.project_edit_task_board_settings.workflow_id, workflow_id);
    assert_eq!(app.project_edit_task_board_settings.clone().sanitized().workflow_id, workflow_id);

    let _ = super::handle(&mut app, crate::app::message::project::ProjectMessage::ProjectEditSaved);
    let saved = app.recent_projects_meta.iter().find(|meta| meta.path == path).unwrap();
    assert_eq!(
        saved.task_board_settings.as_ref().map(|settings| settings.workflow_id.as_str()),
        Some(workflow_id)
    );
    assert_eq!(app.task_board_settings.workflow_id, workflow_id);
}

#[test]
fn project_rejects_unknown_workflow_id() {
    let _guard = RuntimeWorkflowGuard::with_workflows(Vec::new());
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditWorkflowIdChanged(
            "missing_workflow".to_string(),
        ),
    );

    assert_eq!(
        app.project_edit_task_board_settings.workflow_id,
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID
    );

    app.project_edit_task_board_settings.workflow_id = "missing_workflow".to_string();
    assert_eq!(
        app.project_edit_task_board_settings.clone().sanitized().workflow_id,
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID
    );
}

#[test]
fn edit_icon_file_picked_ignores_empty_and_accepts_trimmed_path() {
    let mut app = app();
    app.project_edit_icon = "old".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconFilePicked(None),
    );
    assert_eq!(app.project_edit_icon, "old");

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconFilePicked(Some(
            "   ".to_string(),
        )),
    );
    assert_eq!(app.project_edit_icon, "old");

    app.project_edit_icon_hovered = true;
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconFilePicked(Some(
            " /tmp/icon.png ".to_string(),
        )),
    );
    assert_eq!(app.project_edit_icon, "/tmp/icon.png");
    assert!(!app.project_edit_icon_hovered);
}

#[test]
fn edit_icon_pick_file_returns_async_picker_task() {
    let mut app = app();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditIconPickFile,
    );

    assert!(task.is_some());
}

#[test]
fn edit_toggles_and_numeric_changes_clamp_settings() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoRefreshToggled(false),
    );
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditSessionAutoRefreshToggled(false),
    );

    assert!(!app.project_edit_task_board_auto_refresh);
    assert!(!app.project_edit_task_board_settings.auto_refresh);
    assert!(!app.project_edit_session_auto_refresh);

    let messages = [
        crate::app::message::project::ProjectMessage::ProjectEditMaxConcurrentChanged(99),
        crate::app::message::project::ProjectMessage::ProjectEditSessionRefreshIntervalSecondsChanged(0),
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsChanged(9999),
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(99),
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(0),
        crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMinutesChanged(9999),
        crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(99),
        crate::app::message::project::ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(0),
        crate::app::message::project::ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(1),
    ];
    for message in messages {
        let _ = super::handle(&mut app, message);
    }

    assert_eq!(app.project_edit_task_board_settings.max_concurrent, 10);
    assert_eq!(app.project_edit_session_refresh_interval_seconds_input, "1");
    assert_eq!(app.project_edit_task_board_settings.refresh_interval_seconds, 3600);
    assert_eq!(app.project_edit_task_board_settings.scheduler_tick_interval_seconds, 60);
    assert_eq!(app.project_edit_task_board_settings.auto_promote_tick_interval_seconds, 1);
    assert_eq!(app.project_edit_task_board_settings.failed_retry_minutes, 1440);
    assert_eq!(app.project_edit_task_board_settings.failed_retry_max_attempts, 10);
    assert_eq!(app.project_edit_task_board_settings.running_timeout_minutes, 1);
    assert_eq!(app.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds, 5);
}

#[test]
fn edit_input_messages_preserve_raw_text() {
    let mut app = app();

    let inputs = [
        crate::app::message::project::ProjectMessage::ProjectEditMaxConcurrentInputChanged("x".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged("2".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged("3".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged("4".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged("5".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMinutesInputChanged("6".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMaxAttemptsInputChanged("10".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditRunningTimeoutMinutesInputChanged("7".to_string()),
        crate::app::message::project::ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged("8".to_string()),
    ];
    for message in inputs {
        let _ = super::handle(&mut app, message);
    }

    assert_eq!(app.project_edit_max_concurrent_input, "x");
    assert_eq!(app.project_edit_session_refresh_interval_seconds_input, "2");
    assert_eq!(app.project_edit_task_board_refresh_interval_seconds_input, "3");
    assert_eq!(app.project_edit_task_board_scheduler_tick_interval_seconds_input, "4");
    assert_eq!(app.project_edit_task_board_auto_promote_tick_interval_seconds_input, "5");
    assert_eq!(app.project_edit_failed_retry_minutes_input, "6");
    assert_eq!(app.project_edit_failed_retry_max_attempts_input, "10");
    assert_eq!(app.project_edit_running_timeout_minutes_input, "7");
    assert_eq!(app.project_edit_pr_submitted_stall_timeout_seconds_input, "8");
}

#[test]
fn edit_save_without_path_is_noop_and_runtime_error_sets_error_message() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::ProjectEditSaved);
    assert!(task.is_some());
    assert!(app.project_edit_path.is_none());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditRuntimeSaved(Err(
            "gateway".to_string()
        )),
    );
    assert_eq!(app.error_message.as_deref(), Some("保存项目扩展配置失败: gateway"));
}

#[test]
fn edit_runtime_success_is_silent_and_unhandled_message_is_delegated() {
    let mut app = app();

    let handled = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditRuntimeSaved(Ok(())),
    );
    assert!(handled.is_some());
    assert!(app.error_message.is_none());

    let delegated = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectPathChanged(
            "/tmp/delegated".to_string(),
        ),
    );
    assert!(delegated.is_none());
    assert!(app.project_path.is_none());
}

#[test]
fn edit_save_updates_meta_worktree_and_resets_editor_state() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_path = Some(path.clone());
    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec!["Old".to_string()];
    app.project_edit_path = Some(path.clone());
    app.project_edit_name = "  ".to_string();
    app.project_edit_icon = " icon ".to_string();
    app.project_edit_icon_color = " #123456 ".to_string();
    app.project_edit_start_script = " pnpm dev ".to_string();
    app.project_edit_worktree_enabled = true;
    app.project_edit_max_concurrent_input = "99".to_string();
    app.project_edit_task_board_auto_refresh = false;
    app.project_edit_session_auto_refresh = false;
    app.project_edit_session_refresh_interval_seconds_input = "9999".to_string();
    app.project_edit_task_board_refresh_interval_seconds_input = "bad".to_string();
    app.project_edit_task_board_scheduler_tick_interval_seconds_input = "0".to_string();
    app.project_edit_task_board_auto_promote_tick_interval_seconds_input = "9999".to_string();
    app.project_edit_failed_retry_minutes_input = "0".to_string();
    app.project_edit_failed_retry_max_attempts_input = "99".to_string();
    app.project_edit_running_timeout_minutes_input = "9999".to_string();
    app.project_edit_pr_submitted_stall_timeout_seconds_input = "1".to_string();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::ProjectEditSaved);

    assert!(task.is_some());
    assert_eq!(app.recent_projects_edits, vec!["project".to_string()]);
    let saved = app.recent_projects_meta.iter().find(|item| item.path == path).unwrap();
    assert_eq!(saved.name, "project");
    assert_eq!(saved.icon.as_deref(), Some("icon"));
    assert_eq!(saved.icon_color.as_deref(), Some("#123456"));
    assert_eq!(saved.worktree_start_command.as_deref(), Some("pnpm dev"));
    assert!(!saved.session_auto_refresh);
    assert_eq!(saved.session_refresh_interval_seconds, 3600);
    assert!(app.project_worktree_enabled.contains_key(&path));
    assert!(app.project_edit_path.is_none());
    assert!(app.project_edit_name.is_empty());
    assert!(app.project_edit_icon.is_empty());
    assert_eq!(app.project_edit_tab, crate::app::state::ProjectEditTab::General);
    assert_eq!(app.task_board_settings.max_concurrent, 10);
    assert_eq!(app.task_board_settings.failed_retry_max_attempts, 10);
    assert!(!app.task_board_settings.auto_refresh);
}

#[test]
fn edit_cancel_resets_transient_fields() {
    let mut app = app();
    app.project_edit_path = Some("/tmp/project".to_string());
    app.project_edit_name = "Name".to_string();
    app.project_edit_icon = "icon".to_string();
    app.project_edit_icon_hovered = true;
    app.project_edit_icon_color = "#fff".to_string();
    app.project_edit_icon_color_picker_open = true;
    app.project_edit_start_script = "run".to_string();
    app.project_edit_worktree_enabled = true;
    app.project_edit_max_concurrent_input = "10".to_string();
    app.project_edit_task_board_auto_refresh = false;

    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::ProjectEditCanceled);

    assert!(app.project_edit_path.is_none());
    assert!(app.project_edit_name.is_empty());
    assert!(app.project_edit_icon.is_empty());
    assert!(!app.project_edit_icon_hovered);
    assert!(app.project_edit_icon_color.is_empty());
    assert!(!app.project_edit_icon_color_picker_open);
    assert!(app.project_edit_start_script.is_empty());
    assert!(!app.project_edit_worktree_enabled);
    assert!(app.project_edit_max_concurrent_input.is_empty());
    assert!(app.project_edit_task_board_auto_refresh);
}

#[test]
fn recent_remove_pressed_opens_confirmation_without_removing() {
    let mut app = app();
    let removed = "/tmp/remove".to_string();
    let kept = "/tmp/keep".to_string();
    app.recent_projects = vec![removed.clone(), kept.clone()];
    app.recent_projects_edits = vec!["Remove".to_string(), "Keep".to_string()];
    app.project_tools_menu_path = Some(removed.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemovePressed(removed.clone()),
    );

    assert_eq!(app.recent_projects, vec![removed.clone(), kept]);
    assert_eq!(app.recent_project_remove_confirm_path, Some(removed));
    assert!(app.project_tools_menu_path.is_none());
}

#[test]
fn recent_remove_pressed_allows_current_project_confirmation() {
    let mut app = app();
    let current = "/tmp/current".to_string();
    app.recent_projects = vec![current.clone()];
    app.recent_projects_edits = vec!["Current".to_string()];
    app.project_path = Some(current.clone());
    app.project_tools_menu_path = Some(current.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemovePressed(current.clone()),
    );

    assert_eq!(app.recent_projects, vec![current.clone()]);
    assert_eq!(app.recent_project_remove_confirm_path, Some(current));
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.active_toast.is_none());
}

#[test]
fn recent_remove_canceled_clears_confirmation() {
    let mut app = app();
    app.recent_project_remove_confirm_path = Some("/tmp/remove".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemoveCanceled,
    );

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
    app.project_edit_path = Some(removed.clone());
    app.project_edit_name = "Remove".to_string();
    app.recent_project_remove_confirm_path = Some(removed.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemoveConfirmed(removed),
    );

    assert_eq!(app.recent_projects, vec![kept]);
    assert_eq!(app.recent_projects_edits, vec!["Keep".to_string()]);
    assert!(app.hovered_recent_project.is_none());
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.project_edit_path.is_none());
    assert!(app.project_edit_name.is_empty());
    assert!(app.recent_project_remove_confirm_path.is_none());
}

#[test]
fn recent_remove_confirmed_current_project_selects_first_remaining_project() {
    let mut app = app();
    let current = "/tmp/current".to_string();
    let next = "/tmp/next".to_string();
    app.recent_projects = vec![current.clone(), next.clone()];
    app.recent_projects_edits = vec!["Current".to_string(), "Next".to_string()];
    app.project_path = Some(current.clone());
    app.recent_project_remove_confirm_path = Some(current.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemoveConfirmed(current.clone()),
    );

    assert_eq!(app.recent_projects, vec![next.clone()]);
    assert_eq!(app.project_path, Some(next));
    assert!(app.recent_project_remove_confirm_path.is_none());
    assert!(app.active_toast.is_none());
}

#[test]
fn recent_remove_rebuilds_edit_names_from_file_names_when_meta_is_missing() {
    let mut app = app();
    let removed = "/tmp/remove".to_string();
    let kept = "/tmp/plain-keep".to_string();
    app.recent_projects = vec![removed.clone(), kept.clone()];
    app.project_tools_menu_path = Some("/tmp/other".to_string());
    app.project_edit_path = Some("/tmp/other".to_string());
    app.project_edit_name = "Other".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemoveConfirmed(removed),
    );

    assert_eq!(app.recent_projects, vec![kept]);
    assert_eq!(app.recent_projects_edits, vec!["plain-keep".to_string()]);
    assert_eq!(app.project_tools_menu_path, Some("/tmp/other".to_string()));
    assert_eq!(app.project_edit_path, Some("/tmp/other".to_string()));
    assert_eq!(app.project_edit_name, "Other");
}
