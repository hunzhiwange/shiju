#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("worktree_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

#[test]
fn picker_loaded_reorders_last_directory_after_primary_workspace() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/project-wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_last_directory = Some(worktree.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPickerLoaded {
            project_path: project,
            options: Ok(vec![
                ("/tmp/other".to_string(), "其它".to_string()),
                (worktree.clone(), "最近".to_string()),
                ("/tmp/project".to_string(), "主工作区".to_string()),
            ]),
        },
    );

    assert_eq!(
        app.new_session_picker_options,
        vec![
            (worktree, "最近".to_string()),
            ("/tmp/other".to_string(), "其它".to_string()),
            ("/tmp/project".to_string(), "主工作区".to_string()),
        ]
    );
}

#[test]
fn picker_loaded_reorders_last_directory_after_first_primary_workspace() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/project-wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_last_directory = Some(worktree.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPickerLoaded {
            project_path: project,
            options: Ok(vec![
                ("/tmp/project".to_string(), "主工作区".to_string()),
                ("/tmp/other".to_string(), "其它".to_string()),
                (worktree.clone(), "最近".to_string()),
            ]),
        },
    );

    assert_eq!(app.new_session_picker_options[0].1, "主工作区");
    assert_eq!(app.new_session_picker_options[1].0, worktree);
}

#[test]
fn picker_loaded_ignores_stale_project_and_falls_back_on_error() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/current".to_string());
    app.new_session_picker_options = vec![("keep".to_string(), "保留".to_string())];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPickerLoaded {
            project_path: "/tmp/stale".to_string(),
            options: Ok(vec![("stale".to_string(), "过期".to_string())]),
        },
    );
    assert_eq!(app.new_session_picker_options, vec![("keep".to_string(), "保留".to_string())]);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPickerLoaded {
            project_path: "/tmp/current".to_string(),
            options: Err("gateway down".to_string()),
        },
    );

    assert_eq!(app.error_message.as_deref(), Some("加载工作区列表失败: gateway down"));
    assert_eq!(
        app.new_session_picker_options,
        vec![("__create_worktree__".to_string(), "创建新的独立工作区".to_string())]
    );
}

#[test]
fn picker_close_and_name_change_update_state() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_picker_options = vec![("dir".to_string(), "Dir".to_string())];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionWorktreeNameChanged(
            "feature-login".to_string(),
        ),
    );
    assert_eq!(app.new_session_worktree_name, "feature-login");

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPickerClose,
    );

    assert!(app.hovered_recent_project.is_none());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
}

#[test]
fn picking_existing_workspace_remembers_directory_and_resets_picker_state() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_picker_options =
        vec![("/tmp/project-wt".to_string(), "功能工作区".to_string())];
    app.new_session_worktree_name = "stale-name".to_string();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPicked {
            project_path: "/tmp/project".to_string(),
            directory: "/tmp/project-wt".to_string(),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/tmp/project-wt"));
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
}

#[test]
fn create_worktree_validates_empty_and_invalid_names_before_async_call() {
    let mut app = app();
    let project = "/tmp/project".to_string();

    app.new_session_worktree_name = "   ".to_string();
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionWorktree(project.clone()),
    );
    assert_eq!(app.error_message.as_deref(), Some("请输入 worktree 英文名称"));

    app.error_message = None;
    app.new_session_worktree_name = "Feature_Login".to_string();
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionWorktree(project.clone()),
    );
    assert_eq!(app.error_message.as_deref(), Some("worktree 名称仅支持小写英文、数字、连字符(-)"));

    app.error_message = None;
    app.new_session_picker_project = Some(project.clone());
    app.new_session_worktree_name = "feature-login".to_string();
    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionWorktree(project),
    );
    assert!(task.is_some());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_worktree_name.is_empty());
}

#[test]
fn delete_worktree_blocks_primary_and_sets_confirmation_for_secondary() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/project-wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_picker_options =
        vec![(project.clone(), "主工作区".to_string()), (worktree.clone(), "功能".to_string())];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktree(
            project.clone(),
        ),
    );
    assert_eq!(app.new_session_delete_error.as_deref(), Some("主工作区禁止删除"));
    assert!(app.new_session_confirm_delete_directory.is_none());

    app.new_session_delete_error = Some("old".to_string());
    app.new_session_force_delete_directory = Some("old-dir".to_string());
    app.new_session_confirm_reset_directory = Some("old-reset".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktree(
            worktree.clone(),
        ),
    );

    assert_eq!(app.new_session_confirm_delete_directory, Some(worktree));
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_confirm_reset_directory.is_none());
}

#[test]
fn delete_worktree_cancel_and_missing_confirmation_are_noops() {
    let mut app = app();
    app.new_session_confirm_delete_directory = Some("/tmp/wt".to_string());
    app.new_session_force_delete_directory = Some("/tmp/wt".to_string());
    app.new_session_delete_error = Some("error".to_string());
    app.new_session_reset_error = Some("reset".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeCancel,
    );
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_reset_error.is_none());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed,
    );
    assert!(task.is_some());
}

#[test]
fn delete_worktree_confirmed_requires_project_and_directory() {
    let mut app = app();
    app.new_session_confirm_delete_directory = Some("/tmp/wt".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed,
    );
    assert!(task.is_some());
    assert_eq!(app.new_session_confirm_delete_directory.as_deref(), Some("/tmp/wt"));

    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_confirm_delete_directory = None;
    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed,
    );
    assert!(task.is_some());
    assert!(app.new_session_confirm_delete_directory.is_none());
}

#[test]
fn delete_worktree_confirmed_for_secondary_clears_messages_before_async_task() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_confirm_delete_directory = Some("/tmp/wt".to_string());
    app.new_session_force_delete_directory = Some("/tmp/old-force".to_string());
    app.new_session_delete_error = Some("old delete error".to_string());
    app.new_session_confirm_reset_directory = Some("/tmp/old-reset".to_string());
    app.new_session_reset_error = Some("old reset error".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed,
    );

    assert!(task.is_some());
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn delete_worktree_confirmed_blocks_primary_and_result_handles_success_error_and_stale_project() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_confirm_delete_directory = Some(project.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed,
    );
    assert_eq!(app.new_session_delete_error.as_deref(), Some("主工作区禁止删除"));

    app.new_session_delete_error = Some("old".to_string());
    app.new_session_force_delete_directory = Some("old".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
            project_path: "/tmp/stale".to_string(),
            directory: worktree.clone(),
            result: Ok(()),
        },
    );
    assert_eq!(app.new_session_delete_error.as_deref(), Some("old"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
            project_path: project.clone(),
            directory: worktree.clone(),
            result: Err("dirty worktree".to_string()),
        },
    );
    assert_eq!(app.new_session_delete_error.as_deref(), Some("dirty worktree"));
    assert_eq!(app.new_session_force_delete_directory, Some(worktree.clone()));

    app.new_session_last_directory = Some(worktree.clone());
    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
            project_path: project,
            directory: worktree,
            result: Ok(()),
        },
    );
    assert!(task.is_some());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_last_directory.is_none());
}

#[test]
fn delete_worktree_success_preserves_a_different_last_directory() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_last_directory = Some("/tmp/recent-wt".to_string());
    app.new_session_delete_error = Some("old".to_string());
    app.new_session_force_delete_directory = Some("/tmp/deleted-wt".to_string());
    app.new_session_reset_error = Some("old reset".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
            project_path: project,
            directory: "/tmp/deleted-wt".to_string(),
            result: Ok(()),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/tmp/recent-wt"));
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn force_delete_requires_directory_then_starts_async_task() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeForceConfirmed,
    );
    assert!(app.new_session_force_delete_directory.is_none());

    app.new_session_force_delete_directory = Some("/tmp/wt".to_string());
    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeForceConfirmed,
    );
    assert!(task.is_some());
    assert!(app.new_session_force_delete_directory.is_none());
}

#[test]
fn force_delete_requires_an_active_project() {
    let mut app = app();
    app.new_session_force_delete_directory = Some("/tmp/wt".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionDeleteWorktreeForceConfirmed,
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_force_delete_directory.as_deref(), Some("/tmp/wt"));
}

#[test]
fn reset_worktree_blocks_primary_confirms_secondary_and_cancels() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/wt".to_string();
    app.new_session_picker_project = Some(project.clone());
    app.new_session_picker_options = vec![(project.clone(), "主工作区".to_string())];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktree(project),
    );
    assert_eq!(app.new_session_reset_error.as_deref(), Some("主工作区禁止重置"));
    assert!(app.new_session_confirm_reset_directory.is_none());

    app.new_session_picker_options.clear();
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktree(
            worktree.clone(),
        ),
    );
    assert_eq!(app.new_session_confirm_reset_directory, Some(worktree));
    assert!(app.new_session_reset_error.is_none());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeCancel,
    );
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn reset_worktree_confirmed_and_result_update_state() {
    let mut app = app();
    let project = "/tmp/project".to_string();
    let worktree = "/tmp/wt".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeConfirmed,
    );
    assert!(app.new_session_confirm_reset_directory.is_none());

    app.new_session_picker_project = Some(project.clone());
    app.new_session_confirm_reset_directory = Some(project.clone());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeConfirmed,
    );
    assert_eq!(app.new_session_reset_error.as_deref(), Some("主工作区禁止重置"));

    app.new_session_reset_error = Some("old".to_string());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeResult {
            project_path: "/tmp/stale".to_string(),
            directory: worktree.clone(),
            result: Ok(()),
        },
    );
    assert_eq!(app.new_session_reset_error.as_deref(), Some("old"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeResult {
            project_path: project.clone(),
            directory: worktree.clone(),
            result: Err("reset failed".to_string()),
        },
    );
    assert_eq!(app.new_session_reset_error.as_deref(), Some("reset failed"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeResult {
            project_path: project,
            directory: worktree,
            result: Ok(()),
        },
    );
    assert!(app.new_session_reset_error.is_none());
    assert!(app.notifications.iter().any(|item| item.message == "工作区已重置"));
}

#[test]
fn reset_worktree_confirmed_for_secondary_clears_prompt_before_async_task() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_confirm_reset_directory = Some("/tmp/wt".to_string());
    app.new_session_reset_error = Some("old".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionResetWorktreeConfirmed,
    );

    assert!(task.is_some());
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn unrelated_project_message_is_not_handled() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::OpenFolderPressed);

    assert!(task.is_none());
}
