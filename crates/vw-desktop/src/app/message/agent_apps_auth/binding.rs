pub use desktop_apps_ui::agent_apps_auth::binding::connection_matches;
use super::{Event, message};
use crate::app::{App, Message, components::system_settings::SystemTab, state::app_providers::edit::EditForm};
use iced::Task;
use gateway_endpoint::GatewayEndpoint;
use vw_api_app_identity::AppId;
pub(crate) fn entry(form: &EditForm, app: AppId, generation: u64) -> Option<Message> {
    desktop_apps_ui::agent_apps_auth::binding::entry(form, app, generation).map(message)
}
pub(super) fn update(app: &mut App, event: Event, endpoint: GatewayEndpoint) -> Task<Message> {
    use desktop_apps_ui::agent_apps_auth::binding::{Origin, Effects};
    let origin = Origin {
        visible_apps: app.show_system_settings && app.system_settings_tab == SystemTab::Apps,
        visible_auth: app.show_system_settings && app.system_settings_tab == SystemTab::Auth,
        provider_generation: app.app_providers.generation,
        provider_scope: app.app_providers.scope.as_ref(),
        form: app.app_providers.form.as_ref(),
        project_path: &app.project_path,
        token: &app.apps_catalog.token,
        host: &app.apps_catalog.host,
        catalog_generation: app.apps_catalog.generation,
    };
    let mut notice = None;
    let mut effects = Effects::default();
    let task = desktop_apps_ui::agent_apps_auth::binding::update(&mut app.agent_apps_auth, &mut notice, &mut effects, origin, event, endpoint).map(message);
    if effects.opened { app.system_settings_tab = SystemTab::Auth; app.system_settings_help_tab = None; }
    if effects.saved { app.app_providers.usage.clear(); }
    let toast = match notice {
        Some(desktop_apps_ui::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, toast])
}
