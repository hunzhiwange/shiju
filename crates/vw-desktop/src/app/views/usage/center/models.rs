//! Desktop mapping for model price synchronization.
use crate::app::{App, Message, message::ViewMessage};
pub use desktop_usage::models::{Event, State};
use iced::Element;
fn message(event: Event) -> Message {
    Message::View(ViewMessage::UsageCenter(super::Event::Models(event)))
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    desktop_usage::models::overlay_with_base(&app.usage_center.models, base, message)
}
