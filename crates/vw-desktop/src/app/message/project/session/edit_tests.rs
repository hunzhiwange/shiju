// 宿主保留依据：编辑消息关联项目元数据、运行时工作流注册与保存任务，保留宿主装配契约。
// 验证会话编辑面板、工具菜单与工作流草稿状态。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("edit_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
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
fn edit_save_without_path_is_noop_and_runtime_error_sets_error_message() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::ProjectEditSaved);
    assert!(task.is_some());
    assert!(app.project_editor.project_edit_path.is_none());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectEditRuntimeSaved(Err(
            "gateway".to_string()
        )),
    );
    assert_eq!(app.error_message.as_deref(), Some("保存项目扩展配置失败: gateway"));
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
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
    assert!(app.navigation_tabs.project_path.is_none());
}

#[test]
fn edit_save_updates_meta_worktree_and_resets_editor_state() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.navigation_tabs.project_path = Some(path.clone());
    app.recent_projects = vec![path.clone()];
    app.recent_projects_edits = vec!["Old".to_string()];
    app.project_editor.project_edit_path = Some(path.clone());
    app.project_editor.project_edit_name = "  ".to_string();
    app.project_editor.project_edit_icon = " icon ".to_string();
    app.project_editor.project_edit_icon_color = " #123456 ".to_string();
    app.project_editor.project_edit_start_script = " pnpm dev ".to_string();
    app.project_editor.project_edit_worktree_enabled = true;
    app.project_editor.project_edit_max_concurrent_input = "99".to_string();
    app.project_editor.project_edit_task_board_auto_refresh = false;
    app.project_editor.project_edit_session_auto_refresh = false;
    app.project_editor.project_edit_session_refresh_interval_seconds_input = "9999".to_string();
    app.project_editor.project_edit_task_board_refresh_interval_seconds_input = "bad".to_string();
    app.project_editor.project_edit_task_board_scheduler_tick_interval_seconds_input = "0".to_string();
    app.project_editor.project_edit_task_board_auto_promote_tick_interval_seconds_input = "9999".to_string();
    app.project_editor.project_edit_failed_retry_minutes_input = "0".to_string();
    app.project_editor.project_edit_failed_retry_max_attempts_input = "99".to_string();
    app.project_editor.project_edit_running_timeout_minutes_input = "9999".to_string();
    app.project_editor.project_edit_pr_submitted_stall_timeout_seconds_input = "1".to_string();

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
    assert!(app.project_editor.project_edit_path.is_none());
    assert!(app.project_editor.project_edit_name.is_empty());
    assert!(app.project_editor.project_edit_icon.is_empty());
    assert_eq!(app.project_editor.project_edit_tab, crate::app::state::ProjectEditTab::General);
    assert_eq!(app.task_board_settings.max_concurrent, 10);
    assert_eq!(app.task_board_settings.failed_retry_max_attempts, 10);
    assert!(!app.task_board_settings.auto_refresh);
}



#[test]
fn recent_remove_pressed_allows_current_project_confirmation() {
    let mut app = app();
    let current = "/tmp/current".to_string();
    app.recent_projects = vec![current.clone()];
    app.recent_projects_edits = vec!["Current".to_string()];
    app.navigation_tabs.project_path = Some(current.clone());
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
fn recent_remove_confirmed_current_project_selects_first_remaining_project() {
    let mut app = app();
    let current = "/tmp/current".to_string();
    let next = "/tmp/next".to_string();
    app.recent_projects = vec![current.clone(), next.clone()];
    app.recent_projects_edits = vec!["Current".to_string(), "Next".to_string()];
    app.navigation_tabs.project_path = Some(current.clone());
    app.recent_project_remove_confirm_path = Some(current.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentRemoveConfirmed(current.clone()),
    );

    assert_eq!(app.recent_projects, vec![next.clone()]);
    assert_eq!(app.navigation_tabs.project_path, Some(next));
    assert!(app.recent_project_remove_confirm_path.is_none());
    assert!(app.active_toast.is_none());
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
