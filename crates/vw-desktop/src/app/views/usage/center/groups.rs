//! Desktop composition for the usage groups panel.
use crate::app::{App, Message, message::ViewMessage};
use desktop_usage::center::groups as domain;
pub use desktop_usage::center::groups::{Event, State};
use iced::Element;
fn map(event: Event) -> Message {
    Message::View(ViewMessage::UsageCenter(super::Event::Groups(event)))
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(&app.usage_center.groups).map(map)
}
