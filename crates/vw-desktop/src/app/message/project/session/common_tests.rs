#[test]
fn clear_new_session_picker_messages_preserves_picker_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.new_session_picker_project = Some("/project".to_string());
    app.new_session_picker_options = vec![("/project".to_string(), "Main".to_string())];
    app.new_session_worktree_name = "feature".to_string();
    app.new_session_last_directory = Some("/project/previous".to_string());
    app.new_session_confirm_delete_directory = Some("/project/wt".to_string());
    app.new_session_force_delete_directory = Some("/project/wt".to_string());
    app.new_session_delete_error = Some("delete failed".to_string());
    app.new_session_confirm_reset_directory = Some("/project/wt".to_string());
    app.new_session_reset_error = Some("reset failed".to_string());

    super::clear_new_session_picker_messages(&mut app);

    assert_eq!(app.new_session_picker_project.as_deref(), Some("/project"));
    assert_eq!(app.new_session_picker_options.len(), 1);
    assert_eq!(app.new_session_worktree_name, "feature");
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/project/previous"));
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn reset_new_session_picker_state_clears_selection_options_and_messages() {
    let (mut app, _task) = crate::app::App::new();
    app.new_session_picker_project = Some("/project".to_string());
    app.new_session_picker_options = vec![("/project".to_string(), "Main".to_string())];
    app.new_session_worktree_name = "feature".to_string();
    app.new_session_last_directory = Some("/project/previous".to_string());
    app.new_session_confirm_delete_directory = Some("/project/wt".to_string());
    app.new_session_force_delete_directory = Some("/project/wt".to_string());
    app.new_session_delete_error = Some("delete failed".to_string());
    app.new_session_confirm_reset_directory = Some("/project/wt".to_string());
    app.new_session_reset_error = Some("reset failed".to_string());

    super::reset_new_session_picker_state(&mut app);

    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
    assert_eq!(app.new_session_last_directory.as_deref(), Some("/project/previous"));
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_force_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
    assert!(app.new_session_confirm_reset_directory.is_none());
    assert!(app.new_session_reset_error.is_none());
}
