//! Desktop composition for the usage display panel.
use crate::app::{App, Message, message::ViewMessage};
pub use desktop_usage::center::display::{Event, State};
use desktop_usage::center::{Outcome, display as domain};
use iced::Element;
pub(super) fn map(outcome: Outcome<Event>) -> Message {
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
    domain::overlay(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), &app.usage_center.display, super::identity(), base, map)
}
pub fn amount(app: &App, usd: &str) -> String {
    domain::amount(&app.usage_center.display, super::identity(), usd)
}
