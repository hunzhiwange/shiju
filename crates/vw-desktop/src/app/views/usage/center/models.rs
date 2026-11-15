//! Desktop mapping for model price synchronization.
pub use desktop_usage::models::{State, Event};
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};
fn message(event: Event) -> Message { Message::View(ViewMessage::UsageCenter(super::Event::Models(event))) }
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::SavedNotice) { return app.show_success_toast("模型同步选择已保存"); }
    let current = super::identity(app);
    desktop_usage::models::update(&mut app.usage_center.models, current, crate::app::config::gateway_client, event).map(message)
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    desktop_usage::models::overlay_with_base(&app.usage_center.models, base, message)
}
