use crate::project_message::session_common;
#[derive(Default)]
struct PickerFixture {
    new_session_picker_project: Option<String>,
    new_session_picker_options: Vec<(String, String)>,
    new_session_worktree_name: String,
    new_session_last_directory: Option<String>,
    new_session_confirm_delete_directory: Option<String>,
    new_session_force_delete_directory: Option<String>,
    new_session_delete_error: Option<String>,
    new_session_confirm_reset_directory: Option<String>,
    new_session_reset_error: Option<String>,
}
impl PickerFixture {
    fn clear_messages(&mut self) {
        session_common::clear_picker_messages(
            &mut self.new_session_confirm_delete_directory,
            &mut self.new_session_force_delete_directory,
            &mut self.new_session_delete_error,
            &mut self.new_session_confirm_reset_directory,
            &mut self.new_session_reset_error,
        );
    }
    fn reset(&mut self) {
        session_common::reset_picker_selection(
            &mut self.new_session_picker_project,
            &mut self.new_session_picker_options,
            &mut self.new_session_worktree_name,
        );
        self.clear_messages();
    }
}
// Picker 选择、消息与最后目录的组合不变量。
#[test]
fn clear_new_session_picker_messages_preserves_picker_selection() {
    let mut app = PickerFixture::default();
    app.new_session_picker_project = Some("/project".to_string());
    app.new_session_picker_options = vec![("/project".to_string(), "Main".to_string())];
    app.new_session_worktree_name = "feature".to_string();
    app.new_session_last_directory = Some("/project/previous".to_string());
    app.new_session_confirm_delete_directory = Some("/project/wt".to_string());
    app.new_session_force_delete_directory = Some("/project/wt".to_string());
    app.new_session_delete_error = Some("delete failed".to_string());
    app.new_session_confirm_reset_directory = Some("/project/wt".to_string());
    app.new_session_reset_error = Some("reset failed".to_string());

    app.clear_messages();

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
    let mut app = PickerFixture::default();
    app.new_session_picker_project = Some("/project".to_string());
    app.new_session_picker_options = vec![("/project".to_string(), "Main".to_string())];
    app.new_session_worktree_name = "feature".to_string();
    app.new_session_last_directory = Some("/project/previous".to_string());
    app.new_session_confirm_delete_directory = Some("/project/wt".to_string());
    app.new_session_force_delete_directory = Some("/project/wt".to_string());
    app.new_session_delete_error = Some("delete failed".to_string());
    app.new_session_confirm_reset_directory = Some("/project/wt".to_string());
    app.new_session_reset_error = Some("reset failed".to_string());

    app.reset();

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

#[test]
fn history_open_rejects_stale_target_and_preserves_remote_reset_boundary() {
    use crate::project_message::session::history_open::*;
    let ids = vec![Some("kept".to_owned())];
    let mut open = HistoryOpen {
        active_session: Some("current"),
        target_session: "current",
        current_generation: true,
        source: HistorySource::Remote,
        current_ids: &ids,
        reset_boundary: Some(&ids),
        current_signature: 5,
    };
    assert_eq!(open.accept(Some("other")), HistoryDecision::Discard);
    assert_eq!(open.accept(Some("current")), HistoryDecision::Replace);
    assert_eq!(open.accept(None), HistoryDecision::ReportRemoteError);
    assert_eq!(
        open.content(&[Some("old".into()), Some("kept".into())], 6),
        HistoryDecision::KeepCurrent
    );
    assert_eq!(open.content(&ids, 5), HistoryDecision::RefreshMetadata);
    open.source = HistorySource::UiStore;
    assert_eq!(open.accept(None), HistoryDecision::LoadRemote);
    assert_eq!(open.content(&ids, 5), HistoryDecision::Replace);
    open.current_generation = false;
    assert_eq!(open.accept(Some("current")), HistoryDecision::Discard);
    assert_eq!(open.accept(None), HistoryDecision::Discard);
}

mod worktree_tests;

mod interaction_tests;

mod edit_tests;

mod list_tests;

mod membership_tests;

mod overlay_tests;
