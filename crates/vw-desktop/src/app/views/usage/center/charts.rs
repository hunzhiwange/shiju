//! Desktop composition for the usage charts panel.
use crate::app::{App, Message, message::ViewMessage};
use desktop_usage::center::charts as domain;
use desktop_usage::center::charts::Event;
pub use desktop_usage::center::charts::State;
use iced::Element;
fn map(event: Event) -> Message {
    Message::View(ViewMessage::UsageCenter(super::Event::Charts(event)))
}
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    domain::view(i18n, &app.usage_center.charts).map(map)
}
