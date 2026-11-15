//! Desktop composition for the usage groups panel.
pub use desktop_usage::center::groups::{State, Event};
use desktop_usage::center::groups as domain;
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};
fn map(event: Event) -> Message { Message::View(ViewMessage::UsageCenter(super::Event::Groups(event))) }
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = super::identity(app);
    let visible = app.screen == crate::app::Screen::Usage;
    domain::update(&mut app.usage_center.groups, current, visible, app.usage_center.open, app.usage_center.sequence, event).map(map)
}
pub fn view(app: &App) -> Element<'_, Message> { domain::view(&app.usage_center.groups).map(map) }
pub fn load(ticket: u64, scope: super::Identity, query: vw_api_usage::UsageQuery, time_zone: String) -> Task<Message> { domain::load(crate::app::config::gateway_client(), ticket, scope, query, time_zone).map(map) }
