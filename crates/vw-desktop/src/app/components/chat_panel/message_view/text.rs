pub use desktop_chat_view::message_view::text::{SessionControlSelection, user_message_preview, estimate_text_height, split_text_segments, should_segment_text_block, task_session_visible_text, split_session_control_selection, should_prefer_plain_think_body, estimate_message_height_rough};
use crate::app::{App, Message, message::ChatMessage};
use iced::{Color, Element};
pub(super) fn map_message(message: desktop_chat_view::message_view::text::Message) -> Message {
    use desktop_chat_view::message_view::text::Message as Local;
    Message::Chat(match message {
        Local::MessageEditorAction(idx, action) => ChatMessage::MessageEditorAction(idx, action),
        Local::ThinkEditorAction(msg, idx, action) => ChatMessage::ThinkEditorAction(msg, idx, action),
    })
}
pub(super) fn message_text_body<'a>(content: String, is_user: bool) -> Element<'a, Message> {
    desktop_chat_view::message_view::text::message_text_body(content, is_user).map(map_message)
}
pub(super) fn session_control_selection_card<'a>(i18n: crate::app::i18n::I18n, selection: SessionControlSelection) -> Element<'a, Message> {
    desktop_chat_view::message_view::text::session_control_selection_card(i18n, selection).map(map_message)
}
pub(super) fn message_editor_body<'a>(app: &'a App, idx: usize, value: Color) -> Option<Element<'a, Message>> {
    desktop_chat_view::message_view::text::message_editor_body(&app.chat_message_editors, idx, value).map(|view| view.map(map_message))
}
pub(super) use self::message_editor_body as user_message_editor_body;
pub(super) fn think_text_body<'a>(app: &'a App, msg_idx: usize, think_idx: usize, content: String, prefer_plain_text: bool) -> Element<'a, Message> {
    desktop_chat_view::message_view::text::think_text_body(&app.chat_think_editors, msg_idx, think_idx, content, prefer_plain_text).map(map_message)
}
