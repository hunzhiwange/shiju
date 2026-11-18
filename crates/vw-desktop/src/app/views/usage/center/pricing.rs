//! Desktop composition for the usage pricing panel.
use crate::app::{App, Message, message::ViewMessage};
pub use desktop_usage::center::pricing::{Event, State};
use desktop_usage::center::{Outcome, pricing as domain};
use iced::Element;
fn map(outcome: Outcome<Event>) -> Message {
    match outcome {
        Outcome::Event(Event::None) => Message::None,
        Outcome::Event(Event::Help(effect)) => {
            crate::app::components::text_editor_context_menu::text_context_message(effect)
        }
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
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    domain::overlay(&app.usage_center.pricing, super::identity(app), base, i18n, map)
}
