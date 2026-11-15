//! Read-only discovery and market search share one request generation.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;

pub(crate) use desktop_settings_skills::market_view::{Results, Event};
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsMarket(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    use desktop_settings_skills::update::market_view::{Effect, update as reduce};
    let input = super::update_input(app);
    match reduce(&mut app.skills_settings.market, &input, event) {
        Effect::None => Task::none(),
        Effect::Error(error) => app.show_error_toast(error),
        Effect::Request { generation, project, host, query } => {
            let token = app.apps_catalog.token.clone();
            Task::perform(crate::app::session_gateway::gateway_skills_market_request(
                project, host, query, token),
                move |result| message(Event::Completed(generation, result)))
        }
    }
}
