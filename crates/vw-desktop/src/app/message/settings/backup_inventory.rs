//! Authenticated catalog projection and notification mapping.
use crate::app::{App, Message, state::AgentAppsContext};
use super::SettingsMessage;
use iced::{Element, Task};
pub(crate) use desktop_settings_memory::backup_inventory::{Event, State, Identity};
use desktop_settings_memory::backup_inventory::{self as domain};
fn identity(app: &App) -> Option<Identity> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    Some(Identity { gateway: context.gateway, tenant: context.tenant, project_path: context.project, project: vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).to_owned(),
        host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation })
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::NotifyError(error) = event { return app.show_error_toast(error); }
    let current = identity(app);
    domain::update(&mut app.apps_catalog.backups, current, crate::app::config::gateway_client().ok(), event)
        .map(|event| Message::Settings(SettingsMessage::BackupInventory(event)))
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    domain::view(&app.apps_catalog.backups, identity(app)).map(|event| Message::Settings(SettingsMessage::BackupInventory(event)))
}
