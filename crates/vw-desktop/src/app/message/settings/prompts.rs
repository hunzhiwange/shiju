//! Desktop connection and toast adapter for domain prompt updates.
use crate::app::{App, Message};
use iced::Task;
pub use desktop_settings_skills::prompts::update::{Event, pi_files, shared_import, sync, templates, transfer};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(super::SettingsMessage::AppsCatalog(super::AppsCatalogMessage::Prompts(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Toast(error, text) = event {
        return if error { app.show_error_toast(text) } else { app.show_success_toast(text) };
    }
    let mut input = desktop_settings_skills::prompts::update::Input {
        client: crate::app::config::gateway_client(),
        project_id: app.workspace_state.project_id.clone(), project_path: app.navigation_tabs.project_path.clone(),
        context: app.apps_catalog.context.clone(), generation: app.apps_catalog.generation,
        host: app.apps_catalog.host.clone(), token: app.apps_catalog.token.clone(),
        prompts: &mut app.apps_catalog.prompts,
    };
    desktop_settings_skills::prompts::update::update(&mut input, event).map(message)
}
