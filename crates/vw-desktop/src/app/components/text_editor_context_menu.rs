pub use desktop_ui_controls::text_editor_context_menu::*;

/// 将控件局部效果接入桌面消息。
pub(crate) fn text_context_message(
    effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect,
) -> crate::app::Message {
    use crate::app::{Message, message::{ChatMessage, ViewMessage}};
    use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
    match effect {
        TextContextEffect::Copy(text) => Message::CopyCode(text),
        TextContextEffect::Append(text) => Message::Chat(ChatMessage::AppendText(text)),
        TextContextEffect::OpenUrl(url) => Message::View(ViewMessage::OpenUrlExternal(url)),
    }
}
