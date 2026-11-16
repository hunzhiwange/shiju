// Cross-domain draft installation and host message classification.
fn app() -> crate::app::App {
    crate::app::App::new().0
}

#[test]
fn desktop_first_response_picking_existing_workspace_opens_a_scoped_draft() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/project".to_string());
    app.workspace_state.new_session_picker_project = Some("/tmp/project".to_string());
    app.workspace_state.new_session_picker_options =
        vec![("/tmp/project-wt".to_string(), "功能工作区".to_string())];
    app.workspace_state.new_session_worktree_name = "stale-name".to_string();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSessionPicked {
            project_path: "/tmp/project".to_string(),
            directory: "/tmp/project-wt".to_string(),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.workspace_state.new_session_last_directory.as_deref(), Some("/tmp/project-wt"));
    assert!(app.workspace_state.new_session_picker_project.is_none());
    assert!(app.workspace_state.new_session_picker_options.is_empty());
    assert!(app.workspace_state.new_session_worktree_name.is_empty());
    assert!(!app.workspace_state.new_session_creating);
    assert!(app.session_state.active_session_id.is_none());
    assert!(app.session_state.sessions.is_empty());
    let draft = app.current_session_runtime().draft.unwrap();
    assert_eq!(draft.directory, "/tmp/project-wt");
    assert_eq!(draft.project_path, "/tmp/project");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_project_message_is_not_handled() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::OpenFolderPressed);

    assert!(task.is_none());
}
