//! Desktop composition for the application editor.
use crate::app::{App, Message, state::AgentAppsContext};
use desktop_apps_ui::{Connection, Notice, agent_apps::openclaw_model as domain};
use iced::{Element, Task};
pub(crate) use domain::State;
pub use domain::{Event, Scope};
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::OpenClawModel(event)))
}
pub(crate) fn entry() -> Element<'static, Message> { domain::entry().map(message) }
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !domain::is_open(state) { return base; }
    iced::widget::stack![base, domain::overlay(state, iced::widget::Space::new().into()).map(message)].into()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) { domain::close(&mut app.apps_catalog.openclaw_model); return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|id| !id.is_empty()) else { return app.show_error_toast("请先选择项目"); };
    let connection = Connection { client, context, catalog_context: app.apps_catalog.context.clone(), project,
        host: app.apps_catalog.host.clone(), generation: app.apps_catalog.generation, token: app.apps_catalog.token.clone() };
    let mut notice = None;
    let task = domain::update(&mut app.apps_catalog.openclaw_model, connection, event, &mut notice).map(message);
    let feedback = match notice {
        Some(Notice::Error(text)) => app.show_error_toast(text),
        Some(Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
