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
    if !app.usage_center.scan.open {
        return base;
    }
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let current = super::identity();
    let display =
        desktop_usage::center::display::quick_settings(i18n, &app.usage_center.display, current.as_ref())
            .map(super::display::map);
    domain::overlay(i18n, &app.usage_center.scan, current, base, display, map)
}
pub use domain::{Action, Reply};
