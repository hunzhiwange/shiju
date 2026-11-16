//! Desktop composition for the usage logs panel.
use crate::app::{App, Message, message::ViewMessage};
use desktop_usage::center::logs as domain;
pub use desktop_usage::center::logs::{Event, State};
use iced::Element;
fn map(event: Event) -> Message {
    Message::View(ViewMessage::UsageCenter(super::Event::Logs(event)))
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(&app.usage_center.logs).map(map)
}
