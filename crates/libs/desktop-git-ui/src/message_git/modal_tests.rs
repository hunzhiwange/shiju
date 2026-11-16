//! 原局部状态断言，使用仅含被测领域字段的中性夹具。
use crate::message_git::messages::GitMessage;
use crate::state::State as Fixture;
use iced::widget::text_editor;
fn test_app() -> Fixture {
    Fixture::default()
}
fn update(app: &mut Fixture, message: GitMessage) {
    let _ = crate::message_git::controller::modal::update(
        app,
        &Default::default(),
        &mut Vec::new(),
        message,
    );
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
