//! Desktop connection and message mapping for the domain editor.
use crate::app::{App, Message, state::AgentAppsContext};
use desktop_apps_ui::{Connection, Notice, agent_apps::hermes_dashboard as domain};
pub(crate) use domain::State;
pub use domain::{Event, Scope};
use iced::{Element, Task};
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::HermesDashboard(event),
    ))
}
pub(crate) fn view(state: &State) -> Element<'_, Message> {
    domain::view(state).map(message)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::OpenUrl(url) = event {
        return Task::done(Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url)));
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，请重新连接");
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.navigation_tabs.project_path.clone(),
    };
    let Some(project) = app.project_id.clone().filter(|id| !id.is_empty()) else {
        return app.show_error_toast("请先选择项目");
    };
    let connection = Connection {
        client,
        context,
        catalog_context: app.apps_catalog.context.clone(),
        project,
        host: app.apps_catalog.host.clone(),
        generation: app.apps_catalog.generation,
        token: app.apps_catalog.token.clone(),
    };
    let mut notice = None;
    let task =
        domain::update(&mut app.apps_catalog.hermes_dashboard, connection, event, &mut notice)
            .map(message);
    let feedback = match notice {
        Some(Notice::Error(text)) => app.show_error_toast(text),
        Some(Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
