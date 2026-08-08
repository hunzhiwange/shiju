use super::{GitMessage, update};
use crate::app::{App, Message};
use iced::Task;
use std::io::Write;

fn test_app() -> App {
    App::new().0
}

async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("modal_tests"));
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

#[test]
fn custom_diff_result_hides_inputs_and_shows_diff_view() {
    let mut app = test_app();
    app.active_preview_path = Some("/tmp/a.rs".to_string());
    app.show_diff = false;

    let _ = update(
        &mut app,
        GitMessage::OpenCustomDiffResult {
            title: "Patch".to_string(),
            before: "old".to_string(),
            after: "new".to_string(),
        },
    );

    assert!(app.show_git_custom_diff_modal);
    assert!(app.git_custom_diff_hide_inputs);
    assert_eq!(app.git_custom_diff_title, "Patch");
    assert_eq!(app.git_custom_diff_before_editor.text(), "old");
    assert_eq!(app.git_custom_diff_after_editor.text(), "new");
    assert_eq!(app.active_preview_path, None);
    assert!(app.show_diff);
}

#[test]
fn chat_text_diff_open_and_close_update_panel_state() {
    let mut app = test_app();
    app.chat_text_diff = Some(crate::app::state::ChatTextDiff {
        title: "stale".to_string(),
        file: "stale.rs".to_string(),
        before: "old".to_string(),
        after: "new".to_string(),
    });
    app.active_preview_path = Some("preview.rs".to_string());

    let _ = update(
        &mut app,
        GitMessage::OpenChatTextDiff {
            title: "Chat patch".to_string(),
            file: "main.rs".to_string(),
            before: "a".to_string(),
            after: "b".to_string(),
        },
    );

    assert!(app.chat_text_diff.is_none());
    assert!(app.show_git_custom_diff_modal);
    assert!(app.git_custom_diff_hide_inputs);
    assert_eq!(app.git_custom_diff_title, "Chat patch");
    assert_eq!(app.git_custom_diff_before_editor.text(), "a");
    assert_eq!(app.git_custom_diff_after_editor.text(), "b");
    assert!(app.show_diff);
    assert!(app.file_manager_show_changes);
    assert!(app.active_preview_path.is_none());

    let _ = update(&mut app, GitMessage::CloseChatTextDiff);
    assert!(app.chat_text_diff.is_none());
    assert!(!app.show_git_custom_diff_modal);
    assert!(!app.git_custom_diff_hide_inputs);
}

#[test]
fn copy_modal_with_short_text_initializes_both_editors_in_colored_mode() {
    let mut app = test_app();

    let _ = update(&mut app, GitMessage::OpenCopyModalWithText("diff --git a/a b/a".to_string()));

    assert!(app.show_git_copy_modal);
    assert!(app.git_copy_modal_use_color);
    assert_eq!(app.git_copy_modal_editor.text(), "diff --git a/a b/a");
    assert_eq!(app.git_copy_modal_code_editor.content(), "diff --git a/a b/a");
}

#[test]
fn copy_modal_with_text_uses_plain_editor_for_large_text() {
    let mut app = test_app();
    let large = "x".repeat(260_000);

    let _ = update(&mut app, GitMessage::OpenCopyModalWithText(large.clone()));

    assert!(app.show_git_copy_modal);
    assert!(!app.git_copy_modal_use_color);
    assert_eq!(app.git_copy_modal_editor.text(), large);
}

#[test]
fn copy_modal_color_toggle_falls_back_for_large_text() {
    let mut app = test_app();
    let large = "x".repeat(260_000);
    let _ = update(&mut app, GitMessage::OpenCopyModalWithText(large));

    let _ = update(&mut app, GitMessage::ToggleCopyModalColored(true));

    assert!(!app.git_copy_modal_use_color);
}

#[test]
fn copy_modal_color_toggle_on_initializes_code_editor_from_plain_text() {
    let mut app = test_app();
    app.git_copy_modal_use_color = false;
    app.git_copy_modal_editor = iced::widget::text_editor::Content::with_text("plain source");

    let _ = update(&mut app, GitMessage::ToggleCopyModalColored(true));

    assert!(app.git_copy_modal_use_color);
    assert_eq!(app.git_copy_modal_code_editor.content(), "plain source");
}

#[test]
fn copy_modal_color_toggle_off_restores_code_editor_content_to_plain_editor() {
    let mut app = test_app();
    let _ = update(&mut app, GitMessage::OpenCopyModalWithText("colored source".to_string()));
    app.git_copy_modal_editor = iced::widget::text_editor::Content::with_text("stale plain text");

    let _ = update(&mut app, GitMessage::ToggleCopyModalColored(false));

    assert!(!app.git_copy_modal_use_color);
    assert_eq!(app.git_copy_modal_editor.text(), "colored source");
}

#[test]
fn copy_modal_plain_editor_action_updates_its_content() {
    let mut app = test_app();
    app.git_copy_modal_editor = iced::widget::text_editor::Content::new();

    let _ = update(
        &mut app,
        GitMessage::CopyModalEditorAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Insert('x'),
        )),
    );

    assert_eq!(app.git_copy_modal_editor.text(), "x");
}

#[test]
fn diff_copy_mode_without_project_opens_empty_modal_without_repository_access() {
    let mut app = test_app();
    app.project_path = None;

    let _ = update(&mut app, GitMessage::OpenDiffCopyMode("src/lib.rs".to_string()));

    assert!(app.show_git_copy_modal);
    assert!(app.git_copy_modal_editor.text().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_copy_mode_with_non_repository_project_falls_back_to_empty_patch() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let mut app = test_app();
    app.project_path = Some(directory.path().to_string_lossy().into_owned());

    let _ = update(&mut app, GitMessage::OpenDiffCopyMode("src/lib.rs".to_string()));

    assert!(app.show_git_copy_modal);
    assert!(app.git_copy_modal_editor.text().is_empty());
}

#[test]
fn copy_modal_from_path_reads_utf8_content() {
    let mut file = tempfile::NamedTempFile::new().expect("temporary file");
    file.write_all("from disk".as_bytes()).expect("write fixture");
    let mut app = test_app();

    let _ = update(
        &mut app,
        GitMessage::OpenCopyModalFromPath(file.path().to_string_lossy().into_owned()),
    );

    assert!(app.show_git_copy_modal);
    assert_eq!(app.git_copy_modal_editor.text(), "from disk");
}

#[test]
fn copy_modal_from_missing_path_falls_back_to_empty_content() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let missing = directory.path().join("missing.diff");
    let mut app = test_app();

    let _ =
        update(&mut app, GitMessage::OpenCopyModalFromPath(missing.to_string_lossy().into_owned()));

    assert!(app.show_git_copy_modal);
    assert!(app.git_copy_modal_editor.text().is_empty());
}

#[test]
fn copy_modal_close_hides_modal() {
    let mut app = test_app();
    app.show_git_copy_modal = true;

    let _ = update(&mut app, GitMessage::CloseCopyModal);
    assert!(!app.show_git_copy_modal);
}

#[tokio::test(flavor = "multi_thread")]
async fn copy_current_emits_content_from_plain_and_colored_editors() {
    let mut app = test_app();
    app.git_copy_modal_use_color = false;
    app.git_copy_modal_editor = iced::widget::text_editor::Content::with_text("plain");

    let plain = collect_task_messages(update(&mut app, GitMessage::CopyModalCopyCurrent)).await;
    assert!(matches!(plain.as_slice(), [Message::CopyCode(text)] if text == "plain"));

    let _ = update(&mut app, GitMessage::OpenCopyModalWithText("colored".to_string()));
    let colored = collect_task_messages(update(&mut app, GitMessage::CopyModalCopyCurrent)).await;
    assert!(matches!(colored.as_slice(), [Message::CopyCode(text)] if text == "colored"));
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_current_emits_plain_editor_content_and_hides_modal() {
    let mut app = test_app();
    app.show_git_copy_modal = true;
    app.git_copy_modal_use_color = false;
    app.git_copy_modal_editor = iced::widget::text_editor::Content::with_text("plain chat text");

    let messages =
        collect_task_messages(update(&mut app, GitMessage::InsertCopyModalToChatCurrent)).await;

    assert!(!app.show_git_copy_modal);
    assert!(matches!(
        messages.as_slice(),
        [Message::Chat(crate::app::message::ChatMessage::AppendText(text))]
            if text == "plain chat text"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_current_emits_colored_editor_content_and_hides_modal() {
    let mut app = test_app();
    let _ = update(&mut app, GitMessage::OpenCopyModalWithText("colored chat text".to_string()));

    let messages =
        collect_task_messages(update(&mut app, GitMessage::InsertCopyModalToChatCurrent)).await;

    assert!(!app.show_git_copy_modal);
    assert!(matches!(
        messages.as_slice(),
        [Message::Chat(crate::app::message::ChatMessage::AppendText(text))]
            if text == "colored chat text"
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_explicit_text_emits_that_text_and_hides_modal() {
    let mut app = test_app();
    app.show_git_copy_modal = true;

    let messages = collect_task_messages(update(
        &mut app,
        GitMessage::InsertCopyModalToChat("hello".to_string()),
    ))
    .await;

    assert!(!app.show_git_copy_modal);
    assert!(matches!(
        messages.as_slice(),
        [Message::Chat(crate::app::message::ChatMessage::AppendText(text))] if text == "hello"
    ));
}
