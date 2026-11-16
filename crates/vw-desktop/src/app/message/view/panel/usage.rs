use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_usage::panel as domain;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::UsageCenter(event) => return crate::app::views::usage::center::update(app, event),
        ViewMessage::UsageStepToggled(step_index) => domain::Message::UsageStepToggled(step_index),
        ViewMessage::UsageStepRawLoaded(session_id, step_index, result) => domain::Message::UsageStepRawLoaded(session_id, step_index, result),
        ViewMessage::UsageStepRawOpen(step_index) => domain::Message::UsageStepRawOpen(step_index),
        ViewMessage::UsageModelInfoLoaded(info) => domain::Message::UsageModelInfoLoaded(info),
        ViewMessage::UsageTurnStepToggled(_turn_id, step_index) => domain::Message::UsageTurnStepToggled(_turn_id, step_index),
        ViewMessage::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result) => domain::Message::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result),
        ViewMessage::UsageTurnStepRawOpen(turn_id, step_index) => domain::Message::UsageTurnStepRawOpen(turn_id, step_index),
        ViewMessage::UsageStepRawClose => domain::Message::UsageStepRawClose,
        _ => return Task::none(),
    };
    let effect = domain::update(&mut app.usage_panel, app.active_session_id.as_deref(), &app.active_session_view_state.steps, &app.active_session_view_state.step_llm_raw, message);
    match effect {
        Some(domain::Effect::InsertRaw(key, raw)) => { app.insert_active_session_step_llm_raw(key, raw); Task::none() }
        Some(domain::Effect::LoadRaw((session_id, turn_id, step_index))) => {
            let directory = app.known_session_directory(&session_id).or_else(|| app.navigation_tabs.project_path.clone());
            Task::perform(async move {
                let result = crate::app::session_gateway::gateway_session_step_llm_raw_async(&session_id, &turn_id, step_index, directory.as_deref()).await;
                (session_id, turn_id, step_index, result)
            }, |(session_id, turn_id, step_index, result)| Message::View(ViewMessage::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result)))
        }
        None => Task::none(),
    }
}
#[cfg(test)]
#[path = "usage_tests.rs"]
mod usage_tests;
