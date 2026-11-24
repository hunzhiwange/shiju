//! Explicit Claude controls, isolated from plugin manifest activation.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;

pub(crate) use desktop_settings_skills::update::claude_view::{Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsClaudeConfig(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::claude_view::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.claude_config, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_message_toast(crate::app::state::ToastKind::Error, desktop_i18n::settings_extensions::feedback_message(&error)),
        Effect::Success(notice) => app.show_message_toast(crate::app::state::ToastKind::Success, desktop_i18n::settings_extensions::feedback_message(&notice)),
        Effect::Request { generation, scope, change } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(async move { desktop_session::session_gateway::gateway_claude_config_request(crate::app::config::gateway_client_endpoint(), scope, token, change).await },
                move |result| message(Event::Completed(generation, result)))
        }
    }
}
