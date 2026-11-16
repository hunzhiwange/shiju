//! Desktop composition for the usage pricing panel.
use crate::app::{App, Message, message::ViewMessage};
pub use desktop_usage::center::pricing::{Event, State};
use desktop_usage::center::{Outcome, pricing as domain};
use iced::Element;
fn map(outcome: Outcome<Event>) -> Message {
    match outcome {
        Outcome::Event(event) => {
            Message::View(ViewMessage::UsageCenter(super::Event::Pricing(event)))
        }
        Outcome::Refresh => Message::View(ViewMessage::UsageCenter(super::Event::Refresh)),
        Outcome::Error(message) => Message::View(ViewMessage::UsageCenter(super::Event::Pricing(
            Event::Feedback(false, message),
        ))),
        Outcome::Success(message) => Message::View(ViewMessage::UsageCenter(
            super::Event::Pricing(Event::Feedback(true, message)),
        )),
    }
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    domain::overlay(&app.usage_center.pricing, super::identity(app), base, map)
}
