//! Desktop composition for the usage logs panel.
pub use desktop_usage::center::logs::{State, Event};
use desktop_usage::center::logs as domain;
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};
fn map(event: Event) -> Message { Message::View(ViewMessage::UsageCenter(super::Event::Logs(event))) }
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = super::identity(app);
    let visible = app.screen == crate::app::Screen::Usage;
    domain::update(&mut app.usage_center.logs, current, visible, app.usage_center.open, app.usage_center.sequence, crate::app::config::gateway_client(), event).map(map)
}
pub fn view(app: &App) -> Element<'_, Message> { domain::view(&app.usage_center.logs).map(map) }
pub fn reset(app: &mut App, query: vw_api_usage::UsageQuery) -> Task<Message> {
 let current = super::identity(app);
 let visible = app.screen == crate::app::Screen::Usage;
 domain::reset(&mut app.usage_center.logs, current, visible, app.usage_center.open, app.usage_center.sequence, crate::app::config::gateway_client(), query).map(map)
}
