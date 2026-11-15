use crate::app::{App, Message, message::SettingsMessage};
use iced::{Element, Task};
use vw_api_transport::ProjectScope;
use vw_api_app_operations::DataManagementTarget;
pub(crate) use desktop_settings_memory::system_settings_data::migration_preview_ui::{Event, State};
use desktop_settings_memory::system_settings_data::migration_preview_ui as domain;
pub(super) fn target(app: &App) -> Option<DataManagementTarget> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if !app.apps_catalog.connection_open || app.apps_catalog.context.as_ref() != Some(&context)
        || app.apps_catalog.host.is_empty() || app.apps_catalog.token.is_empty() { return None; }
    Some(DataManagementTarget {
        scope: ProjectScope::new(context.tenant, app.project_id.clone()?.into()).ok()?,
        target_host_id: app.apps_catalog.host.clone(),
        principal_id: app.apps_catalog.migration_preview.principal.trim().to_owned(),
    })
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::NotifyError(error) = event { return app.show_error_toast(error); }
    let target = target(app);
    domain::update(&mut app.apps_catalog.migration_preview, event, target, crate::app::config::gateway_client().ok(), app.apps_catalog.generation, app.apps_catalog.token.clone())
        .map(|event| Message::Settings(SettingsMessage::MigrationPreview(event)))
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let context = crate::app::config::gateway_client().ok().map(|c| (c.endpoint().base_url(), app.apps_catalog.generation));
    domain::view(&app.apps_catalog.migration_preview, target(app), context)
        .map(|event| Message::Settings(SettingsMessage::MigrationPreview(event)))
}
