use super::{App, Message, message};
use iced::Element;
pub(crate) fn with_chat_reset_dialog<'a>(app: &App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_chat_view::modals::chat_reset::with_chat_reset_dialog(app.chat_reset_menu_idx, app.chat.len(), app.desktop_locale, |event| {
        use desktop_chat_view::modals::chat_reset::Event;
        Message::Chat(match event {
            Event::CloseResetMenu => message::ChatMessage::CloseResetMenu,
            Event::ResetSessionToMessage { msg_idx } => message::ChatMessage::ResetSessionToMessage { msg_idx },
        })
    }, root)
}
