//! Desktop context validation and effect mapping for lifecycle updates.
pub use desktop_settings_apps::system_settings_apps::lifecycle_update::*;
use super::{AppsCatalogMessage, SettingsMessage};
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
pub(super) fn update(app: &mut App, message: LifecycleMessage) -> Task<Message> {
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(_) => return app.show_error_toast("网关不可用"),
    };
    let current = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    if app.apps_catalog.reset_if_context_changed(&current) { return Task::none(); }
    let host = app.apps_catalog.host.clone();
    let token = app.apps_catalog.token.clone();
    if host.is_empty() || token.is_empty() { return Task::none(); }
    let mut notice = None;
    let mut invalidate = false;
    let task = desktop_settings_apps::system_settings_apps::lifecycle_update::update(
        &mut app.apps_catalog.lifecycle, &app.apps_catalog.rows,
        client, current, host, token, message, &mut notice, &mut invalidate,
    ).map(|event| Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::Lifecycle(event))));
    if invalidate { app.apps_catalog.loaded_at = None; }
    let feedback = match notice {
        Some(desktop_settings_apps::aggregation::Notice::Error(error)) => app.show_error_toast(error),
        _ => Task::none(),
    };
    Task::batch([task, feedback])
}
