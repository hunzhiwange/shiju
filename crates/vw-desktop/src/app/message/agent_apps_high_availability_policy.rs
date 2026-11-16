//! Desktop connection and notification adapter.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
pub use desktop_apps_ui::agent_apps_high_availability_policy::{State, Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::HighAvailability(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.high_availability.invalidate();
        return Task::none();
    }
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(error) => return app.show_error_toast(error),
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    let requested_app_exists = match &event {
        Event::Open(id) => app.apps_catalog.rows.iter().any(|row| row.entry.id == *id),
        _ => false,
    };
    let mut notice = None;
    let task = desktop_apps_ui::agent_apps_high_availability_policy::update(
        desktop_apps_ui::agent_apps_high_availability_policy::Input {
            state: &mut app.apps_catalog.high_availability, client, context,
            catalog_generation: app.apps_catalog.generation,
            catalog_context: app.apps_catalog.context.as_ref(), project: app.project_id.as_deref(),
            host: &app.apps_catalog.host, token: &app.apps_catalog.token, requested_app_exists,
        }, event, &mut notice).map(message);
    let toast = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, toast])
}
