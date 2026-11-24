//! Desktop connection and message mapping for the domain editor.
use crate::app::{App, Message, state::AgentAppsContext};
use desktop_apps_ui::{Connection, Notice, agent_apps::hermes_dashboard as domain};
pub(crate) use domain::State;
pub use domain::Event;
use iced::{Element, Task};
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::HermesDashboard(event),
    ))
}
pub(crate) fn view(i18n: desktop_i18n::I18n, state: &State) -> Element<'_, Message> {
    domain::view(i18n, state).map(message)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::OpenUrl(url) = event {
        return Task::done(Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url)));
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_localized_toast(crate::app::state::ToastKind::Error, "网关不可用，请重新连接", vec![]);
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.navigation_tabs.project_path.clone(),
    };
    let project = vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).to_owned();
    let connection = Connection {
        client,
        context: context.clone(),
        catalog_context: Some(context.clone()),
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
        Some(Notice::Error(text)) => app.show_message_toast(crate::app::state::ToastKind::Error, text),
        Some(Notice::Success(text)) => app.show_message_toast(crate::app::state::ToastKind::Success, text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
