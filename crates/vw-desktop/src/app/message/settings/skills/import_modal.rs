//! Unmanaged imports use server selections and explicit application choices.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;

pub(crate) use desktop_settings_skills::import_modal::{Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsImport(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::import_modal::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.import, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Refresh => super::refresh_task(app.navigation_tabs.project_path.clone()),
        Effect::Request { generation, project, host, apps, selected } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(async move { desktop_session::session_gateway::gateway_skills_import_request(crate::app::config::gateway_client_endpoint(),
                project, host, apps, selected, token).await },
                move |(outcomes, result)| message(Event::Completed(generation, outcomes, result)))
        }
    }
}
