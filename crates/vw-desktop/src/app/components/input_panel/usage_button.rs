use crate::app::{App, Message, message};
pub fn usage_button(app: &App) -> iced::Element<'_, Message> {
 desktop_chat_input::input_panel::usage_button::usage_button(super::usage::input(app), crate::app::i18n::I18n::new(app.general_settings.desktop_locale))
 .map(|_| Message::View(message::ViewMessage::OpenUsage))
}
