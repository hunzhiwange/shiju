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
    domain::view(&app.usage_center.charts).map(map)
}
