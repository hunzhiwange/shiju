//! One explicit host/source request per confirmation; credentials never enter events.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_api_app_skills::inventory_api::SkillInventoryResponse;

pub(crate) use desktop_settings_skills::install_view::{Event};
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsInstall(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::install_view::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.install, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Refresh => Task::batch(vec![update(app, Event::Refresh), super::refresh_task(app.navigation_tabs.project_path.clone())]),
        Effect::Submitted(scope, id) => super::jobs_view::submitted(app, scope, id),
        Effect::Request { g, scope, request } => {
            let token = app.apps_catalog.token.clone();
            if let Some(request) = request {
                return Task::perform(crate::app::session_gateway::gateway_skills_submit(request, token),
                    move |r| message(Event::Submitted(g, r)));
            }
            Task::perform(async move {
                crate::app::session_gateway::gateway_skills_install_view(scope, None, token).await
            }, move |result| match result {
                Ok(Some(data)) => message(Event::Loaded(g, Ok(data))),
                Ok(None) => message(Event::Installed(g, Ok(()))),
                Err(e) => message(Event::Installed(g, Err(e))),
            })
        }
    }
}
/// Only expose inventory from the active authenticated catalog generation.
pub(crate) fn sync_inventory(app: &App) -> Option<&SkillInventoryResponse> {
    desktop_settings_skills::update::install_view::sync_inventory(&app.skills_settings.install, &app.apps_catalog.context, app.apps_catalog.generation)
}
