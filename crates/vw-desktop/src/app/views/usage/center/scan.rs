//! Desktop composition for the usage scan panel.
use crate::app::{App, Message, message::ViewMessage};
pub use desktop_usage::center::scan::{Event, State};
use desktop_usage::center::{Outcome, scan as domain};
use iced::Element;
fn map(outcome: Outcome<Event>) -> Message {
    match outcome {
        Outcome::Event(event) => Message::View(ViewMessage::UsageCenter(super::Event::Scan(event))),
        Outcome::Refresh => Message::View(ViewMessage::UsageCenter(super::Event::Refresh)),
        Outcome::Error(message) => Message::View(ViewMessage::UsageCenter(super::Event::Scan(
            Event::Feedback(false, message),
        ))),
        Outcome::Success(message) => Message::View(ViewMessage::UsageCenter(super::Event::Scan(
            Event::Feedback(true, message),
        ))),
    }
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    domain::overlay(&app.usage_center.scan, super::identity(app), base, map)
}
pub use domain::{Action, Reply};
