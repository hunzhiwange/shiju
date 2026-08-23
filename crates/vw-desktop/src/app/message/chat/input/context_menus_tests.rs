//! 验证输入框和消息上下文菜单的选择、剪贴板与搜索操作。
//!
//! 编辑器夹具先建立真实选择区间，再验证复制、剪切、粘贴和关闭菜单的状态同步。

#![allow(unused_must_use)]
use super::super::ClipboardPastePayload;
use super::{
    handle_append_context_menu_text, handle_clipboard_paste_resolved,
    handle_close_fork_session_dialog, handle_close_input_context_menu,
    handle_close_message_context_menu, handle_close_reset_menu, handle_copy_context_menu_text,
    handle_copy_input_selection, handle_cut_input_selection, handle_open_fork_session_dialog,
    handle_open_input_context_menu, handle_open_message_context_menu, handle_paste_into_input,
    handle_search_context_menu_with_baidu, handle_search_context_menu_with_bing,
    handle_search_context_menu_with_google, handle_select_all_input, handle_toggle_reset_menu,
};
use crate::app::{App, Message};
use iced::widget::text_editor;

fn test_app() -> App {
    App::new().0
}

fn selected_editor(text: &str) -> text_editor::Content {
    let mut editor = text_editor::Content::with_text(text);
    editor.perform(text_editor::Action::SelectAll);
    editor
}

#[test]
fn context_menus_tests_module_is_wired() {
    assert!(module_path!().ends_with("context_menus_tests"));
}

#[test]
fn input_context_menu_open_close_and_select_all_sync_editor() {
    let mut app = test_app();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("select me");

    handle_open_input_context_menu(&mut app, 12.0, 24.0);

    assert!(app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, Some((12.0, 24.0)));

    handle_select_all_input(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
    assert_eq!(app.current_session_runtime().input_editor.text(), "select me");
    assert_eq!(app.input_editor.text(), "select me");

    handle_open_input_context_menu(&mut app, 1.0, 2.0);
    handle_close_input_context_menu(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
}

#[test]
fn input_context_menu_copy_cut_and_paste_handle_selection_state() {
    let mut app = test_app();
    app.current_session_runtime_mut().input_editor = selected_editor("copy me");
    handle_open_input_context_menu(&mut app, 4.0, 5.0);

    let _task = handle_copy_input_selection(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
    assert_eq!(app.current_session_runtime().input_editor.text(), "copy me");

    app.current_session_runtime_mut().input_editor = selected_editor("cut me");
    app.input_editor = text_editor::Content::with_text("stale");
    handle_open_input_context_menu(&mut app, 6.0, 7.0);

    let _task = handle_cut_input_selection(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
    assert_eq!(app.current_session_runtime().input_editor.text(), "");
    assert_eq!(app.input_editor.text(), "");

    handle_open_input_context_menu(&mut app, 8.0, 9.0);
    let _task = handle_paste_into_input(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
}

#[test]
fn input_context_menu_cut_without_selection_only_closes_menu() {
    let mut app = test_app();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("unchanged");
    handle_open_input_context_menu(&mut app, 2.0, 3.0);

    let _task = handle_cut_input_selection(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
    assert_eq!(app.current_session_runtime().input_editor.text(), "unchanged");
}

#[test]
fn input_context_menu_copy_without_selection_only_closes_menu() {
    let mut app = test_app();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("unchanged");
    handle_open_input_context_menu(&mut app, 2.0, 3.0);

    let _task = handle_copy_input_selection(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
    assert_eq!(app.current_session_runtime().input_editor.text(), "unchanged");
}

#[test]
fn clipboard_paste_resolved_updates_attachments_or_notifications() {
    let mut app = test_app();
    let image_path =
        std::env::temp_dir().join(format!("vw-desktop-clipboard-paste-{}.png", std::process::id()));
    std::fs::write(&image_path, b"image").unwrap();

    handle_clipboard_paste_resolved(
        &mut app,
        ClipboardPastePayload::AttachmentPath(image_path.to_string_lossy().to_string()),
    );

    assert_eq!(app.files.len(), 1);
    assert!(app.files[0].ends_with(".png"));
    assert!(std::path::Path::new(&app.files[0]).exists());
    let _ = std::fs::remove_file(image_path);

    handle_clipboard_paste_resolved(
        &mut app,
        ClipboardPastePayload::Error("bad paste".to_string()),
    );

    assert_eq!(app.notifications.last().map(|item| item.message.as_str()), Some("bad paste"));

    let _task =
        handle_clipboard_paste_resolved(&mut app, ClipboardPastePayload::Text("hello".into()));
    let _task = handle_clipboard_paste_resolved(&mut app, ClipboardPastePayload::Empty);
}

#[test]
fn message_context_menu_copy_append_and_search_trim_text() {
    let mut app = test_app();

    handle_open_message_context_menu(&mut app, 42, 8.0, 9.0, "  rust lang  ".to_string());

    assert_eq!(app.chat_context_menu_target, Some(42));
    assert_eq!(app.chat_context_menu_pos, Some((8.0, 9.0)));
    assert_eq!(app.chat_context_menu_text, "  rust lang  ");

    let _task = handle_copy_context_menu_text(&mut app);
    assert_eq!(app.chat_context_menu_target, None);
    assert_eq!(app.chat_context_menu_pos, None);
    assert!(app.chat_context_menu_text.is_empty());

    app.chat_context_menu_text = " append me ".to_string();
    let _task = handle_append_context_menu_text(&mut app);
    assert!(app.chat_context_menu_text.is_empty());

    for (handler, expected) in [
        (
            handle_search_context_menu_with_baidu as fn(&mut App) -> iced::Task<Message>,
            "https://www.baidu.com/s?wd=",
        ),
        (handle_search_context_menu_with_google, "https://www.google.com/search?q="),
        (handle_search_context_menu_with_bing, "https://www.bing.com/search?q="),
    ] {
        app.chat_context_menu_text = "rust lang".to_string();
        let _task = handler(&mut app);
        assert!(
            app.chat_context_menu_text.is_empty(),
            "expected {expected} search to clear menu text"
        );
    }

    app.chat_context_menu_text = "   ".to_string();
    let _task = handle_copy_context_menu_text(&mut app);
    let _task = handle_append_context_menu_text(&mut app);
    app.chat_context_menu_text = "   ".to_string();
    let _task = handle_search_context_menu_with_google(&mut app);
    assert!(app.chat_context_menu_text.is_empty());
}

#[test]
fn message_context_menu_open_keeps_multiline_fallback_text() {
    let mut app = test_app();
    let fallback = "line1\nline2\nline3".to_string();

    handle_open_message_context_menu(&mut app, 42, 1.0, 2.0, fallback.clone());

    assert_eq!(app.chat_context_menu_text, fallback);
}

#[test]
fn message_context_menu_prefers_active_text_selection_over_fallback_text() {
    let mut app = test_app();
    app.chat_message_editors.push(selected_editor("selected message"));

    handle_open_message_context_menu(&mut app, 0, 3.0, 4.0, "fallback".to_string());

    assert_eq!(app.chat_context_menu_target, Some(0));
    assert_eq!(app.chat_context_menu_pos, Some((3.0, 4.0)));
    assert_eq!(app.chat_context_menu_text, "selected message");
}

#[test]
fn message_context_menu_close_and_reset_dialog_state_are_local() {
    let mut app = test_app();

    handle_open_message_context_menu(&mut app, 7, 1.0, 2.0, "text".to_string());
    handle_close_message_context_menu(&mut app);

    assert_eq!(app.chat_context_menu_target, None);
    assert_eq!(app.chat_context_menu_pos, None);
    assert!(app.chat_context_menu_text.is_empty());

    handle_open_fork_session_dialog(&mut app, 3);
    assert_eq!(app.chat_fork_dialog_idx, Some(3));
    assert_eq!(app.chat_reset_menu_idx, None);

    handle_toggle_reset_menu(&mut app, 4);
    assert_eq!(app.chat_fork_dialog_idx, None);
    assert_eq!(app.chat_reset_menu_idx, Some(4));

    handle_toggle_reset_menu(&mut app, 4);
    assert_eq!(app.chat_reset_menu_idx, None);

    handle_open_fork_session_dialog(&mut app, 2);
    handle_close_fork_session_dialog(&mut app);
    assert_eq!(app.chat_fork_dialog_idx, None);

    app.chat_reset_menu_idx = Some(9);
    handle_close_reset_menu(&mut app);
    assert_eq!(app.chat_reset_menu_idx, None);
}
