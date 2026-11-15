//! Desktop composition for the usage scan panel.
pub use desktop_usage::center::scan::{State, Event};
use desktop_usage::center::{scan as domain, Outcome};
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};
fn map(outcome: Outcome<Event>) -> Message {
    match outcome {
        Outcome::Event(event) => Message::View(ViewMessage::UsageCenter(super::Event::Scan(event))),
        Outcome::Refresh => Message::View(ViewMessage::UsageCenter(super::Event::Refresh)),
        Outcome::Error(message) => Message::View(ViewMessage::UsageCenter(super::Event::Scan(Event::Feedback(false, message)))),
        Outcome::Success(message) => Message::View(ViewMessage::UsageCenter(super::Event::Scan(Event::Feedback(true, message)))),
    }
}
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback(success, message) = event { return if success { app.show_success_toast(message) } else { app.show_error_toast(message) }; }
    let current = super::identity(app);
    domain::update(&mut app.usage_center.scan, current, crate::app::config::gateway_client(), event).map(map)
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> { domain::overlay(&app.usage_center.scan, super::identity(app), base, map) }
pub use domain::{Action, Reply};
pub fn activity(app: &App, active: bool) -> Task<Message> { domain::activity(&app.usage_center.scan, super::identity(app), crate::app::config::gateway_client(), active).map(map) }
