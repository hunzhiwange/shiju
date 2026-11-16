//! Cross-domain contracts for attachment assembly and clipboard routing.
use super::*;
use crate::app::models::ChatRenderCacheEntry;
fn keep_message_view(view: Element<'_, Message>) {
    std::hint::black_box(view);
}
#[test]
fn message_view_builds_attachment_card_for_user_message() {
    let app = crate::app::App::new().0;
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::User,
        "please review\n\n[DOCUMENT:/tmp/spec.pdf]",
        &[],
        None,
        None,
        &cache,
        true,
    );

    keep_message_view(view);
}

#[test]
fn markdown_selected_text_routes_to_clipboard() {
    use iced::futures::StreamExt;
    let mut app = App::new().0;
    app.active_session_id = Some("markdown-selection".to_owned());
    app.navigation_tabs.screen = crate::app::Screen::Project;
    let expected = "文档 和 粗体，这段中文会在窄窗口中换行，继续选择下面的内容。\n\n名称\t值\n数据\t42\n\npnpm install";
    let _ = app.update(Message::ChatMarkdownSelectionChanged {
        session_id: "markdown-selection".to_owned(),
        message_idx: 0,
        text_idx: 0,
        text: Some(expected.to_owned()),
    });
    let task = app.update(Message::ClipboardShortcutRouted {
        kind: crate::app::ui_types::ClipboardShortcutKind::Copy,
        input_focused: false,
    });
    let mut stream = iced_runtime::task::into_stream(task).expect("copy task");
    assert!(
        matches!(iced::futures::executor::block_on(stream.next()),Some(iced_runtime::Action::Output(Message::CopyCode(text))) if text==expected)
    );
}
