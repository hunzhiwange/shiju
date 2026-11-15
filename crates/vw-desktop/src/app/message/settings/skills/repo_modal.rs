//! Versioned repository editing; no optimistic list updates or discovery orchestration.
use crate::app::{App, Message, message::SettingsMessage};
use crate::app::session_gateway::{gateway_skill_repos_request, gateway_skill_repos_scope};
use iced::Task;
use vw_api_app_skills::repos_api::SkillReposResponse;

pub(crate) use desktop_settings_skills::repo_modal::{Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillRepositories(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::repo_modal::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.repositories, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Request { generation, added, directory, host, snapshot, change } => {
    Task::perform(async move {
        let scope = match snapshot.as_ref() {
            Some(s) => s.scope.clone(),
            None => gateway_skill_repos_scope(&directory, host).await?,
        };
        gateway_skill_repos_request(scope, snapshot.map_or(0, |s| s.version), change).await
    }, move |result| message(Event::Completed { generation, added, result }))
        }
    }
}
/// A repository selection must belong to the active gateway, project and host.
pub(crate) fn sync_inventory(app: &App) -> Option<&SkillReposResponse> {
    desktop_settings_skills::update::repo_modal::sync_inventory(&app.skills_settings.repositories, &app.apps_catalog.context, app.project_id.as_deref(), &app.apps_catalog.host)
}
