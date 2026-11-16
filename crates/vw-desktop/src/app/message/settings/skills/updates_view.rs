//! Scoped update results; refresh errors never replace a completed operation.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_api_app_skills::updates_api::CheckSkillUpdatesRequest;

pub(crate) use desktop_settings_skills::updates_view::{Operation, Outcome, Event};
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsUpdates(event)) }

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::updates_view::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.updates, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Refresh => Task::batch(vec![update(app, Event::Refresh), super::refresh_task(app.navigation_tabs.project_path.clone())]),
        Effect::Request { g, scope, operation } => {
            let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
            let token = app.apps_catalog.token.clone();
            if let Some(operation) = operation {
                return Task::perform(async move { desktop_session::session_gateway::gateway_skills_mutation(crate::app::config::gateway_client_endpoint(), scope, operation, token).await }, move |r| message(Event::Finished(g, r)));
            }
            Task::perform(async move {
                let inventory = client.agent_apps_skills_inventory(&scope, &token).await.map_err(|e| e.to_string());
                let updates = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope }, &token).await.map_err(|e| e.to_string());
                (inventory, updates)
            }, move |(i, u)| message(Event::Loaded(g, i, u)))
        }
    }
}
