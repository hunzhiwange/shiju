use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_usage::panel as domain;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::UsageCenter(event) => {
            if matches!(&event, desktop_usage::center::event::Event::Load(_)) {
                let _ = desktop_navigation::panel::apps::update(desktop_navigation::panel::apps::Context {
                    open_tabs: &mut app.navigation_tabs.open_tabs, active_tab_id: &mut app.navigation_tabs.active_tab_id,
                    screen: &mut app.navigation_tabs.screen, apps_search_query: &mut app.navigation_tabs.apps_search_query,
                }, crate::apps::workflow::WORKFLOW_TOOL_TAB_ID, None, desktop_navigation::panel::apps::Message::OpenUsage);
            }
            return crate::app::views::usage::center::update(app, event);
        }
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
    let effect = domain::update(&mut app.usage_panel, app.session_state.active_session_id.as_deref(), &app.chat_view.active_session_view_state.steps, &app.chat_view.active_session_view_state.step_llm_raw, message);
    match effect {
        Some(domain::Effect::InsertRaw(key, raw)) => { app.insert_active_session_step_llm_raw(key, raw); Task::none() }
        Some(domain::Effect::LoadRaw((session_id, turn_id, step_index))) => {
            let directory = app.known_session_directory(&session_id).or_else(|| app.navigation_tabs.project_path.clone());
            Task::perform(async move {
                let result = desktop_session::session_gateway::gateway_session_step_llm_raw_async(crate::app::config::gateway_client_endpoint(), &session_id, &turn_id, step_index, directory.as_deref()).await;
                (session_id, turn_id, step_index, result)
            }, |(session_id, turn_id, step_index, result)| Message::View(ViewMessage::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result)))
        }
        None => Task::none(),
    }
}
#[cfg(test)]
#[path = "usage_tests.rs"]
mod usage_tests;
