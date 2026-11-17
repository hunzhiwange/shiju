//! Storage drafts never replace the acknowledged execution-host root.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;

pub(crate) use desktop_settings_skills::storage_modal::Event;
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsStorage(event)) }

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::storage_modal::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.storage, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Request { generation, scope, change } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(async move { desktop_session::session_gateway::gateway_skill_storage_request(crate::app::config::gateway_client_endpoint(), scope, token, change).await }, move |result| message(Event::Completed(generation, result)))
        }
    }
}

#[cfg(test)]
pub(crate) use desktop_settings_skills::storage_modal::Readback;
#[cfg(test)]
pub(crate) use desktop_settings_skills::update::storage_modal::Change;
#[cfg(test)]
pub(crate) use desktop_settings_skills::update::storage_modal::open_local;
