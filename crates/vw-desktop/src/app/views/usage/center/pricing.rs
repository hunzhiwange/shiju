//! Desktop composition for the usage pricing panel.
pub use desktop_usage::center::pricing::{State, Event};
use desktop_usage::center::{pricing as domain, Outcome};
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};
fn map(outcome: Outcome<Event>) -> Message {
    match outcome {
        Outcome::Event(event) => Message::View(ViewMessage::UsageCenter(super::Event::Pricing(event))),
        Outcome::Refresh => Message::View(ViewMessage::UsageCenter(super::Event::Refresh)),
        Outcome::Error(message) => Message::View(ViewMessage::UsageCenter(super::Event::Pricing(Event::Feedback(false, message)))),
        Outcome::Success(message) => Message::View(ViewMessage::UsageCenter(super::Event::Pricing(Event::Feedback(true, message)))),
    }
}
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback(success, message) = event { return if success { app.show_success_toast(message) } else { app.show_error_toast(message) }; }
    let current = super::identity(app);
    let visible = app.screen == crate::app::Screen::Usage;
    domain::update(&mut app.usage_center.pricing, current, visible, crate::app::config::gateway_client(), event).map(map)
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> { domain::overlay(&app.usage_center.pricing, super::identity(app), base, map) }
