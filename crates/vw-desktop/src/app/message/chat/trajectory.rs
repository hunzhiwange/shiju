//! 聊天面板轨迹模式的瞬时交互与原始载荷补载。

use super::ChatMessage;
use crate::app::components::chat_panel::trajectory::TrajectoryDetailsAction;
use crate::app::state::ChatPanelMode;
use crate::app::{App, Message};
use iced::Task;

pub(super) fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::ChatPanelModeSelected(mode) => {
            app.chat_panel_mode = mode;
            if mode == ChatPanelMode::Trajectory {
                let Some(session_id) = app.active_session_id.clone() else {
                    return Task::none();
                };
                return refresh_missing_raw_task(app, &session_id, None);
            }
            Task::none()
        }
        ChatMessage::TrajectorySearchChanged(search) => {
            app.trajectory_search_text = search;
            Task::none()
        }
        ChatMessage::TrajectoryRecordSelected(record_id) => {
            app.trajectory_details.select(record_id);
            Task::none()
        }
        ChatMessage::TrajectoryDetailsTabSelected(tab) => {
            app.trajectory_details.apply(TrajectoryDetailsAction::SelectTab(tab));
            Task::none()
        }
        ChatMessage::TrajectoryDetailsClosed => {
            app.trajectory_details.apply(TrajectoryDetailsAction::Close);
            Task::none()
        }
        ChatMessage::TrajectoryRawLoaded { session_id, step_index, result } => {
            apply_raw_result(app, session_id, step_index, result);
            Task::none()
        }
        _ => Task::none(),
    }
}

pub(super) fn refresh_missing_raw_task(
    app: &mut App,
    session_id: &str,
    only_step_index: Option<u32>,
) -> Task<Message> {
    if app.active_session_id.as_deref() != Some(session_id) {
        return Task::none();
    }

    let missing = app
        .active_session_view_state
        .steps
        .iter()
        .filter(|step| step.finished_ms.is_some())
        .filter(|step| only_step_index.is_none_or(|index| index == step.index))
        .map(|step| step.index)
        .filter(|step_index| {
            !app.active_session_view_state.step_llm_raw.contains_key(step_index)
                && !app.trajectory_raw_loading.contains(&(session_id.to_string(), *step_index))
        })
        .collect::<Vec<_>>();

    let mut tasks = Vec::with_capacity(missing.len());
    for step_index in missing {
        let session_id = session_id.to_string();
        let key = (session_id.clone(), step_index);
        app.trajectory_raw_errors.remove(&key);
        app.trajectory_raw_loading.insert(key);
        tasks.push(Task::perform(
            async move {
                let result = crate::app::session_gateway::gateway_session_step_llm_raw_async(
                    &session_id,
                    step_index,
                )
                .await;
                (session_id, step_index, result)
            },
            |(session_id, step_index, result)| {
                Message::Chat(ChatMessage::TrajectoryRawLoaded { session_id, step_index, result })
            },
        ));
    }
    Task::batch(tasks)
}

fn apply_raw_result(
    app: &mut App,
    session_id: String,
    step_index: u32,
    result: Result<Option<serde_json::Value>, String>,
) {
    let key = (session_id.clone(), step_index);
    app.trajectory_raw_loading.remove(&key);

    let is_current_completed_step = app.active_session_id.as_deref() == Some(&session_id)
        && app
            .active_session_view_state
            .steps
            .iter()
            .any(|step| step.index == step_index && step.finished_ms.is_some());
    if !is_current_completed_step {
        return;
    }

    match result {
        Ok(Some(raw)) => {
            app.active_session_view_state.step_llm_raw.entry(step_index).or_insert(raw);
            app.trajectory_raw_errors.remove(&key);
        }
        Ok(None) => {
            app.trajectory_raw_errors
                .insert(key, "原始数据尚未就绪；重新进入轨迹即可重试".to_string());
        }
        Err(error) => {
            app.trajectory_raw_errors.insert(key, error);
        }
    }
}
