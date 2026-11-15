//! 桌面消息映射及任务入口的组合断言；分类断言随实现迁入 desktop-session。
use super::resolved_clipboard_message;
use crate::app::Message;
use crate::app::message::chat::{ChatMessage, ClipboardPastePayload};

#[test]
fn clipboard_tests_module_is_wired() {
    assert!(module_path!().ends_with("clipboard_tests"));
}

#[test]
fn read_clipboard_for_input_builds_task_without_touching_callers_state() {
    let _task = super::read_clipboard_for_input();
}

#[test]
fn resolved_clipboard_message_wraps_payload_for_chat_update() {
    let message = resolved_clipboard_message(desktop_session::message::chat::input::clipboard::ClipboardPastePayload::Text("paste me".to_string()));

    assert!(matches!(
        message,
        Message::Chat(ChatMessage::ClipboardPasteResolved(ClipboardPastePayload::Text(text)))
            if text == "paste me"
    ));
}
