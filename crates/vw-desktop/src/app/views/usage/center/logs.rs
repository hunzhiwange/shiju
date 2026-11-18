//! Desktop composition for the usage logs panel.
use crate::app::{App, Message, message::ViewMessage};
use desktop_usage::center::logs as domain;
pub use desktop_usage::center::logs::{Event, State};
use iced::Element;
pub(super) fn map(event: Event) -> Message {
    match event {
        Event::None => Message::None,
        Event::Help(effect) => crate::app::components::text_editor_context_menu::text_context_message(effect),
        event => Message::View(ViewMessage::UsageCenter(super::Event::Logs(event))),
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    domain::view(&app.usage_center.logs, i18n).map(map)
}
