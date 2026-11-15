use super::types::ConventionalCommitType;
use super::left_panel::Input;
fn test_app(editor: &iced::widget::text_editor::Content) -> Input<'_> {
 Input { desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, show_git_diff_summary: false, git_commit_type: None, git_commit_scope: "", git_commit_message: "", git_commit_description_editor: editor, git_commit_in_progress: false, file_manager_refresh_frame: 0, any_selected: false }
}
fn assert_panel_state(app: &Input<'_>, hint: &'static str, button: super::left_panel::CommitButtonState) {
 let state = super::left_panel::commit_panel_state(app);
 assert_eq!(state.hint, hint);
 assert_eq!(state.button, button);
}
#[test]
fn help_copy_lists_commit_format_and_common_types() {
    assert_eq!(super::left_panel::COMMIT_HELP_TITLE, "约定式提交");

    for expected in [
        "约定式提交格式",
        "<类型>[可选作用域]: <摘要>",
        "[可选详细描述]",
        "核心提交类型",
        "feat:",
        "fix:",
        "docs:",
        "style:",
        "refactor:",
        "perf:",
        "test:",
        "build:",
        "ci:",
        "chore:",
        "revert:",
        "扩展与特殊类型",
        "init:",
        "config:",
        "release:",
        "deploy:",
        "merge:",
        "wip:",
        "typo:",
        "locale:",
    ] {
        assert!(super::left_panel::COMMIT_HELP_TEXT.contains(expected), "{expected}");
    }
}

#[test]
fn commit_panel_state_covers_priority_and_validation_paths() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = test_app(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_in_progress = true;
    app.git_commit_message = "   ";
    assert_panel_state(&app, "正在提交已选变更", super::left_panel::CommitButtonState::InProgress);

    let mut no_selection_app = test_app(&editor);
    no_selection_app.git_commit_message = "cover left panel";
    no_selection_app.git_commit_type = Some(ConventionalCommitType::Feat);
    assert_panel_state(
        &no_selection_app,
        "请选择至少一项变更后再提交",
        super::left_panel::CommitButtonState::NoSelection,
    );

    let mut app = test_app(&editor);
    app.show_git_diff_summary = false;
    app.git_commit_message = "   ";
    app.any_selected = true;
    assert_panel_state(
        &app,
        "填写摘要后即可提交",
        super::left_panel::CommitButtonState::MissingSummary,
    );

    let mut app = test_app(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_type = None;
    app.git_commit_message = "cover left panel";
    app.any_selected = true;
    assert_panel_state(
        &app,
        "选择提交类型后即可提交",
        super::left_panel::CommitButtonState::MissingType,
    );

    let mut app = test_app(&editor);
    app.show_git_diff_summary = false;
    app.git_commit_message = "cover left panel";
    app.any_selected = true;
    assert_panel_state(
        &app,
        "已满足提交条件，可直接提交",
        super::left_panel::CommitButtonState::Ready,
    );
}

#[test]
fn commit_panel_state_is_ready_for_complete_conventional_commit() {
    let editor = iced::widget::text_editor::Content::new();
    let mut app = test_app(&editor);
    app.show_git_diff_summary = true;
    app.git_commit_type = Some(ConventionalCommitType::Fix);
    app.git_commit_message = "  cover complete commit form  ";
    app.any_selected = true;

    assert_panel_state(
        &app,
        "已满足提交条件，可直接提交",
        super::left_panel::CommitButtonState::Ready,
    );
}
