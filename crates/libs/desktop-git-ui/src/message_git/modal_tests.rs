//! 原局部状态断言，使用仅含被测领域字段的中性夹具。
use iced::widget::text_editor;
#[derive(Default)]
struct Fixture {
    git_custom_diff_title: String,
    show_git_custom_diff_modal: bool,
    git_custom_diff_hide_inputs: bool,
    git_custom_diff_before_editor: text_editor::Content,
    git_custom_diff_after_editor: text_editor::Content,
}
fn test_app() -> Fixture { Fixture::default() }
enum GitMessage {
    OpenCustomDiffModal,
    CloseCustomDiffModal,
    CustomDiffTitleChanged(String),
    CustomDiffBeforeEditorAction(text_editor::Action),
    CustomDiffAfterEditorAction(text_editor::Action),
    CustomDiffSwap,
}
fn update(app: &mut Fixture, message: GitMessage) {
    match message {
        GitMessage::OpenCustomDiffModal => { let _ = super::open_custom_diff_modal(&mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs, &mut app.git_custom_diff_title); },
        GitMessage::CloseCustomDiffModal => { let _ = super::close_custom_diff_modal(&mut app.show_git_custom_diff_modal, &mut app.git_custom_diff_hide_inputs); },
        GitMessage::CustomDiffTitleChanged(v0) => { let _ = super::custom_diff_title_changed(&mut app.git_custom_diff_title, v0); },
        GitMessage::CustomDiffBeforeEditorAction(v0) => { let _ = super::custom_diff_before_editor_action(&mut app.git_custom_diff_before_editor, v0); },
        GitMessage::CustomDiffAfterEditorAction(v0) => { let _ = super::custom_diff_after_editor_action(&mut app.git_custom_diff_after_editor, v0); },
        GitMessage::CustomDiffSwap => { let _ = super::custom_diff_swap(&mut app.git_custom_diff_before_editor, &mut app.git_custom_diff_after_editor); },
    }
}
#[test]
fn custom_diff_modal_open_close_and_title_change_update_state() {
    let mut app = test_app();
    app.git_custom_diff_title = " \t ".to_string();

    let _ = update(&mut app, GitMessage::OpenCustomDiffModal);
    assert!(app.show_git_custom_diff_modal);
    assert!(!app.git_custom_diff_hide_inputs);
    assert_eq!(app.git_custom_diff_title, "自定义对比");

    let _ = update(&mut app, GitMessage::CustomDiffTitleChanged("Review".to_string()));
    assert_eq!(app.git_custom_diff_title, "Review");

    app.git_custom_diff_hide_inputs = true;
    let _ = update(&mut app, GitMessage::CloseCustomDiffModal);
    assert!(!app.show_git_custom_diff_modal);
    assert!(!app.git_custom_diff_hide_inputs);
}
#[test]
fn custom_diff_modal_open_preserves_non_blank_title_and_reveals_inputs() {
    let mut app = test_app();
    app.git_custom_diff_title = " Existing title ".to_string();
    app.git_custom_diff_hide_inputs = true;

    let _ = update(&mut app, GitMessage::OpenCustomDiffModal);

    assert_eq!(app.git_custom_diff_title, " Existing title ");
    assert!(!app.git_custom_diff_hide_inputs);
}
#[test]
fn custom_diff_editor_actions_and_swap_move_the_edited_contents() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        GitMessage::CustomDiffBeforeEditorAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Insert('左'),
        )),
    );
    let _ = update(
        &mut app,
        GitMessage::CustomDiffAfterEditorAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Insert('右'),
        )),
    );
    let _ = update(&mut app, GitMessage::CustomDiffSwap);

    assert_eq!(app.git_custom_diff_before_editor.text(), "右");
    assert_eq!(app.git_custom_diff_after_editor.text(), "左");
}
