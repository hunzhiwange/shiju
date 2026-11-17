//! Desktop connection and message mapping for the domain editor.
use crate::app::{App, Message, state::AgentAppsContext};
use desktop_apps_ui::{Connection, Notice, agent_apps::workspace_files as domain};
pub(crate) use domain::State;
pub use domain::Event;
use iced::{Element, Task};
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::WorkspaceFiles(event),
    ))
}
pub(crate) fn entry() -> Element<'static, Message> {
    domain::entry().map(message)
}
pub(crate) fn overlay<'a>(
    state: &'a State,
    host: &'a str,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !domain::is_open(state) {
        return base;
    }
    iced::widget::stack![
        base,
        domain::overlay(state, host, iced::widget::Space::new().into()).map(message)
    ]
    .into()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.workspace_files.invalidate();
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，草稿已保留");
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.navigation_tabs.project_path.clone(),
    };
    let Some(project) = app.workspace_state.project_id.clone().filter(|p| !p.is_empty()) else {
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
        domain::update(&mut app.apps_catalog.workspace_files, connection, event, &mut notice)
            .map(message);
    let feedback = match notice {
        Some(Notice::Error(text)) => app.show_error_toast(text),
        Some(Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}

#[cfg(test)]
pub use domain::Scope;
