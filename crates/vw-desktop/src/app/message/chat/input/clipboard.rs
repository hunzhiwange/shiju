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

fn resolved_clipboard_message(
    payload: desktop_session::message::chat::input::clipboard::ClipboardPastePayload,
) -> Message {
    use desktop_session::message::chat::input::clipboard::ClipboardPastePayload as Payload;
    // Until the shared chat message declaration is available, keep the mapping explicit.
    let payload = match payload {
        Payload::Text(text) => ClipboardPastePayload::Text(text),
        Payload::AttachmentPath(path) => ClipboardPastePayload::AttachmentPath(path),
        Payload::Empty => ClipboardPastePayload::Empty,
        Payload::Error(error) => ClipboardPastePayload::Error(error),
        Payload::LocalizedError { source, arguments } => {
            ClipboardPastePayload::LocalizedError { source, arguments }
        }
    };
    Message::Chat(ChatMessage::ClipboardPasteResolved(payload))
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod clipboard_tests;
