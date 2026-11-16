//! External history catalog; credentials stay in the application connection.
use crate::app::{App, Message, state::AgentAppsContext};
pub use desktop_apps_ui::agent_apps::external_sessions::{Event, Scope, State};
use iced::{Element, Task};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ExternalSessions(event),
    ))
}
pub(crate) fn view(state: &State) -> Element<'_, Message> {
    let loopback =
        crate::app::config::gateway_client().is_ok_and(|client| client.endpoint().is_loopback());
    desktop_apps_ui::agent_apps::external_sessions::view(state, loopback).map(message)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Compression(event) = event {
        return update_compression(app, event);
    }
    use desktop_apps_ui::agent_apps::external_sessions::{HostDispatch, prepare_host_event};
    let local = match prepare_host_event(&mut app.apps_catalog.external_sessions, &event) {
        HostDispatch::Blocked | HostDispatch::Handled => return Task::none(),
        HostDispatch::OpenSettings => {
            return crate::app::message::view::update(
                app,
                crate::app::message::view::ViewMessage::OpenSystemSettingsTab(
                    crate::app::components::system_settings::SystemTab::Apps,
                ),
            );
        }
        HostDispatch::Continue { local } => local,
    };
    if let Some(task) = desktop_apps_ui::agent_apps::external_sessions::navigate(
        &mut app.apps_catalog.external_sessions,
        &event,
    ) {
        return task.map(message);
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return if !local || matches!(&event, Event::Delete(_) | Event::Read(_) | Event::Adjacent(_))
        {
            app.show_error_toast("网关不可用，请重新连接")
        } else {
            Task::none()
        };
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    // A loaded response must still be checked when the current project is absent.
    let project = app.project_id.clone().unwrap_or_default();
    let connection = desktop_apps_ui::Connection {
        client,
        context,
        catalog_context: app.apps_catalog.context.clone(),
        project,
        host: app.apps_catalog.host.clone(),
        generation: app.apps_catalog.generation,
        token: app.apps_catalog.token.clone(),
    };
    if matches!(
        &event,
        Event::Maintenance(_) | Event::Path(_) | Event::Resume(_) | Event::Delete(_)
    ) && app.project_id.is_none()
    {
        return Task::none();
    }
    let verified_local = if matches!(&event, Event::Path(_)) {
        crate::app::components::system_settings_apps::local::launch_directory(
            &connection.client,
            &connection.host,
            &connection.token,
            None,
        )
        .is_some()
    } else {
        false
    };
    let mut notice = None;
    let task = if local {
        desktop_apps_ui::agent_apps::external_sessions::update_local(
            &mut app.apps_catalog.external_sessions,
            &connection,
            event,
            &mut notice,
            verified_local,
        )
    } else {
        desktop_apps_ui::agent_apps::external_sessions::update(
            &mut app.apps_catalog.external_sessions,
            connection,
            event,
            &mut notice,
        )
    }
    .map(message);
    let feedback = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}

fn update_compression(
    app: &mut App,
    event: desktop_apps_ui::agent_apps::external_sessions::compression::Event,
) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return Task::none();
    };
    let Some(project) = app.project_id.clone() else {
        return Task::none();
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let connection = desktop_apps_ui::Connection {
        client,
        context,
        catalog_context: app.apps_catalog.context.clone(),
        project,
        host: app.apps_catalog.host.clone(),
        generation: app.apps_catalog.generation,
        token: app.apps_catalog.token.clone(),
    };
    let mut notice = None;
    let task = desktop_apps_ui::agent_apps::external_sessions::compression::update(
        &mut app.apps_catalog.external_sessions,
        connection,
        &mut notice,
        event,
    )
    .map(message);
    let feedback = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
