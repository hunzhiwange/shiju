//! Desktop composition for the usage display panel.
use crate::app::{App, Message, message::ViewMessage};
pub use desktop_usage::center::display::{Event, State};
use desktop_usage::center::{Outcome, display as domain};
use iced::Element;
fn map(outcome: Outcome<Event>) -> Message {
    match outcome {
        Outcome::Event(event) => {
            Message::View(ViewMessage::UsageCenter(super::Event::Display(event)))
        }
        Outcome::Refresh => Message::View(ViewMessage::UsageCenter(super::Event::Refresh)),
        Outcome::Error(message) => Message::View(ViewMessage::UsageCenter(super::Event::Display(
            Event::Feedback(false, message),
        ))),
        Outcome::Success(message) => Message::View(ViewMessage::UsageCenter(
            super::Event::Display(Event::Feedback(true, message)),
        )),
    }
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    domain::overlay(&app.usage_center.display, super::identity(app), base, map)
}
pub fn amount(app: &App, usd: &str) -> String {
    domain::amount(&app.usage_center.display, super::identity(app), usd)
}
pub fn status(app: &App) -> String {
    domain::status(&app.usage_center.display, super::identity(app))
}
