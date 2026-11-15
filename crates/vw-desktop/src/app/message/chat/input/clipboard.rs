//! 将会话剪贴板载荷映射到桌面消息。
use super::super::{ChatMessage, ClipboardPastePayload};
use crate::app::Message;
use iced::Task;

pub(super) fn read_clipboard_for_input() -> Task<Message> {
    desktop_session::message::chat::input::clipboard::read_clipboard_for_input(
        #[cfg(not(target_arch = "wasm32"))]
        crate::app::message::project::helpers::persist_clipboard_image_attachment,
    )
    .map(resolved_clipboard_message)
}

fn resolved_clipboard_message(payload: ClipboardPastePayload) -> Message {
    Message::Chat(ChatMessage::ClipboardPasteResolved(payload))
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod clipboard_tests;
