use crate::app::{App, Message, state::AgentAppsContext};
use iced::{Element, Task};
use desktop_apps_ui::agent_apps::external_sessions::compression as domain;
pub use domain::{State, Event};
pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let Some(project) = app.project_id.clone() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let connection = desktop_apps_ui::Connection { client, context, catalog_context: app.apps_catalog.context.clone(), project,
        host: app.apps_catalog.host.clone(), generation: app.apps_catalog.generation, token: app.apps_catalog.token.clone() };
    let mut notice = None;
    let task = domain::update(&mut app.apps_catalog.external_sessions, connection, &mut notice, event).map(super::message);
    let feedback = match notice { Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text), Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text), None => Task::none() };
    Task::batch([task, feedback])
}
pub(super) fn view<'a>(state: &'a super::State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.compression.is_open() { return base; }
    iced::widget::stack![base, domain::view(state, iced::widget::Space::new().into()).map(super::message)].into()
}
