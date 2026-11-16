//! Desktop access projection and global message mapping for model diagnostics.
use crate::app::{App, Message, message::app_providers::list::{self, ListMessage}};
use desktop_apps_ui::{Notice, diagnostics::model as domain};
use iced::Task;
pub use domain::Event;
pub(crate) fn message(event: Event) -> Message { list::message(ListMessage::ModelDiagnostics(event)) }
pub(crate) fn invalidate(app: &mut App) {
    if app.apps_catalog.model_diagnostics.open.is_some() && !can_show(app) { app.apps_catalog.model_diagnostics.close(); }
}
pub(crate) fn can_show(app: &App) -> bool {
    let access = domain::Access {
        token: &app.apps_catalog.token, context: &app.apps_catalog.context, project: &app.workspace_state.project_id,
        scope: app.app_providers.scope.as_ref(),
        actual: app.app_providers.scope.as_ref().and_then(|s| list::current(app, s.app).ok().map(|(_, scope)| scope)),
        catalog: app.app_providers.snapshot.as_ref().map(|s| &s.catalog),
        client: crate::app::config::gateway_client(),
        show_apps: app.navigation_panels.show_system_settings && app.navigation_panels.system_settings_tab == crate::app::components::system_settings::SystemTab::Apps,
    };
    domain::can_show(&app.apps_catalog.model_diagnostics, &access)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let access = domain::Access {
        token: &app.apps_catalog.token, context: &app.apps_catalog.context, project: &app.workspace_state.project_id,
        scope: app.app_providers.scope.as_ref(),
        actual: app.app_providers.scope.as_ref().and_then(|s| list::current(app, s.app).ok().map(|(_, scope)| scope)),
        catalog: app.app_providers.snapshot.as_ref().map(|s| &s.catalog),
        client: crate::app::config::gateway_client(),
        show_apps: app.navigation_panels.show_system_settings && app.navigation_panels.system_settings_tab == crate::app::components::system_settings::SystemTab::Apps,
    };
    let mut notice = None;
    let task = domain::update(&mut app.apps_catalog.model_diagnostics, &access, &mut notice, event).map(message);
    let feedback = match notice { Some(Notice::Error(text)) => app.show_error_toast(text), Some(Notice::Success(text)) => app.show_success_toast(text), None => Task::none() };
    Task::batch([task, feedback])
}
