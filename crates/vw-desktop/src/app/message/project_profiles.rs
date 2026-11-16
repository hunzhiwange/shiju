//! Desktop gateway admission and toast mapping for workspace profiles.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
pub type Identity = desktop_workspace::profile_update::Identity<AgentAppsContext>;
pub type Event = desktop_workspace::profile_update::Event<AgentAppsContext>;
pub type ApplyEvent = desktop_workspace::profile_update::ApplyEvent<AgentAppsContext>;
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::ProjectProfiles(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|p| !p.trim().is_empty()) else { return Task::none(); };
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return Task::none(); }
    let catalog = desktop_workspace::profile_update::Catalog {
        tenant: context.tenant.clone(), context, project,
        host: &app.apps_catalog.host, generation: app.apps_catalog.generation,
        connected: app.apps_catalog.connection_open, expanded: app.apps_catalog.expanded,
    };
    let update = desktop_workspace::profile_update::update(&mut app.apps_catalog.project_profiles, &catalog,
        gateway_project_profile::GatewayProjectProfileClient::new(client.transport().clone()), event);
    let toast = match update.toast {
        Some((true, text)) => app.show_error_toast(text),
        Some((false, text)) => app.show_info_toast(text),
        None => Task::none(),
    };
    Task::batch([update.task.map(message), toast])
}
