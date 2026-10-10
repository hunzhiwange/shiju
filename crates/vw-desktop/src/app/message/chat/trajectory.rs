//! 聊天面板轨迹模式的瞬时交互与原始载荷补载。

use super::ChatMessage;
use crate::app::components::chat_panel::trajectory::{
    TrajectoryDetailsAction, trajectory_detail_values,
};
use crate::app::state::ChatPanelMode;
use crate::app::state::SessionStepRawKey;
use crate::app::{App, Message};
use iced::Task;
use std::collections::BTreeSet;
use std::time::Duration;

const MAX_RAW_LOAD_ATTEMPTS: u8 = 3;
const SECOND_RAW_LOAD_DELAY: Duration = Duration::from_millis(300);
const THIRD_RAW_LOAD_DELAY: Duration = Duration::from_millis(900);

pub(super) fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    match message {
        ChatMessage::ChatPanelModeSelected(mode) => {
            sync_controls_session(app);
            app.chat_panel_mode = mode;
            if mode == ChatPanelMode::Trajectory {
                let Some(session_id) = app.active_session_id.clone() else {
                    return Task::none();
                };
                return refresh_trajectory_task(app, session_id);
            }
            Task::none()
        }
        ChatMessage::TrajectorySearchChanged(search) => {
            sync_controls_session(app);
            app.trajectory_search_text = search;
            Task::none()
        }
        ChatMessage::TrajectoryDurationToggled => {
            sync_controls_session(app);
            app.trajectory_controls.toggle_duration_mode();
            Task::none()
        }
        ChatMessage::TrajectoryTurnsToggled => {
            sync_controls_session(app);
            let projection = &app.active_session_view_state.trajectory_projection;
            app.trajectory_controls.toggle_all_turns(projection);
            Task::none()
        }
        ChatMessage::TrajectoryCallsToggled => {
            sync_controls_session(app);
            let projection = &app.active_session_view_state.trajectory_projection;
            app.trajectory_controls.toggle_all_calls(projection);
            Task::none()
        }
        ChatMessage::TrajectoryTurnToggled(turn_index) => {
            sync_controls_session(app);
            app.trajectory_controls.toggle_turn(turn_index);
            Task::none()
        }
        ChatMessage::TrajectoryToolStepToggled(step_index) => {
            sync_controls_session(app);
            app.trajectory_controls.toggle_tool_step(step_index);
            Task::none()
        }
        ChatMessage::TrajectoryRecordSelected(record_id) => {
            sync_controls_session(app);
            let projection = app.active_session_view_state.trajectory_projection.clone();
            let changed = if let Some(record) = projection.record(&record_id) {
                app.trajectory_controls.reveal_record(record);
                app.trajectory_details.select_record(record)
            } else {
                app.trajectory_details.select(record_id)
            };
            if changed {
                sync_details_values_from_projection(app, &projection);
            }
            Task::none()
        }
        ChatMessage::TrajectoryDetailsTabSelected(tab) => {
            if app.trajectory_details.apply(TrajectoryDetailsAction::SelectTab(tab)) {
                sync_details_values(app);
            }
            Task::none()
        }
        ChatMessage::TrajectoryDetailsClosed => {
            app.trajectory_details.apply(TrajectoryDetailsAction::Close);
            Task::none()
        }
        ChatMessage::TrajectoryDetailsEditorAction(index, action) => {
            app.trajectory_details.perform_editor_action(index, action);
            Task::none()
        }
        ChatMessage::TrajectoryDetailsContextMenuOpened { index, x, y } => {
            app.trajectory_details.open_context_menu(index, x, y);
            Task::none()
        }
        ChatMessage::TrajectoryDetailsContextMenuClosed => {
            app.trajectory_details.close_context_menu();
            Task::none()
        }
        ChatMessage::TrajectoryDetailsContextMenuCopy => {
            let text = app.trajectory_details.context_menu_copy_text();
            app.trajectory_details.close_context_menu();
            text.map_or_else(Task::none, |text| Task::done(Message::CopyCode(text)))
        }
        ChatMessage::TrajectoryDetailsJsonToggled(path) => {
            app.trajectory_details.toggle_json_path(path);
            Task::none()
        }
        ChatMessage::TrajectoryDetailsJsonCopied(value) => {
            let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
            Task::done(Message::CopyCode(text))
        }
        ChatMessage::TrajectoryRawRetry { session_id, turn_id, step_index, attempt } => {
            retry_raw_load_task(app, session_id, turn_id, step_index, attempt)
        }
        ChatMessage::TrajectoryRawLoaded { session_id, turn_id, step_index, attempt, result } => {
            apply_raw_result(app, session_id, turn_id, step_index, attempt, result)
        }
        ChatMessage::TrajectorySnapshotLoaded { session_id, result } => {
            apply_snapshot_result(app, session_id, result)
        }
        _ => Task::none(),
    }
}

fn refresh_trajectory_task(app: &mut App, session_id: String) -> Task<Message> {
    let raw_task = refresh_missing_raw_task(app, &session_id, None);
    let snapshot_session_id = session_id.clone();
    let snapshot_task = Task::perform(
        async move {
            crate::app::session_gateway::gateway_load_session_any_with_raw_async(&session_id).await
        },
        move |result| {
            Message::Chat(ChatMessage::TrajectorySnapshotLoaded {
                session_id: snapshot_session_id,
                result,
            })
        },
    );
    Task::batch([raw_task, snapshot_task])
}

fn apply_snapshot_result(
    app: &mut App,
    session_id: String,
    result: Result<Option<vw_gateway_client::SessionUiLoadResult>, String>,
) -> Task<Message> {
    if app.chat_panel_mode != ChatPanelMode::Trajectory
        || app.active_session_id.as_deref() != Some(&session_id)
    {
        return Task::none();
    }
    let Ok(Some(loaded)) = result else {
        return Task::none();
    };
    if loaded.session.id != session_id {
        return Task::none();
    }

    for step in loaded.session.steps {
        let already_present = app.active_session_view_state.steps.iter().any(|existing| {
            existing.index == step.index && existing.turn_id.as_deref() == step.turn_id.as_deref()
        });
        if !already_present {
            app.upsert_active_session_step(step);
        }
    }
    app.active_session_view_state.llm_raw_step_indices.extend(loaded.llm_raw_step_indices);
    app.insert_missing_active_session_step_llm_raw(loaded.step_llm_raw);
    sync_details_values(app);
    refresh_missing_raw_task(app, &session_id, None)
}

fn sync_controls_session(app: &mut App) {
    app.trajectory_controls.sync_session(app.active_session_id.as_deref());
}

fn sync_details_values(app: &mut App) {
    let projection = app.active_session_view_state.trajectory_projection.clone();
    sync_details_values_from_projection(app, &projection);
}

fn sync_details_values_from_projection(
    app: &mut App,
    projection: &crate::app::components::chat_panel::trajectory::TrajectoryProjection,
) {
    let values = app
        .trajectory_details
        .selected_record_id()
        .and_then(|record_id| projection.record(record_id))
        .map(|record| trajectory_detail_values(record, app.trajectory_details.active_tab()))
        .unwrap_or_default();
    app.trajectory_details.sync_values(values);
}

pub(super) fn refresh_missing_raw_task(
    app: &mut App,
    session_id: &str,
    only_step_index: Option<u32>,
) -> Task<Message> {
    if app.active_session_id.as_deref() != Some(session_id) {
        return Task::none();
    }

    app.trajectory_raw_loading
        .retain(|(loading_session_id, _, _)| loading_session_id == session_id);
    app.trajectory_raw_attempts
        .retain(|(loading_session_id, _, _), _| loading_session_id == session_id);
    app.trajectory_raw_errors.retain(|(error_session_id, _, _), _| error_session_id == session_id);

    let mut candidate_keys = app
        .active_session_view_state
        .steps
        .iter()
        .filter(|step| step.finished_ms.is_some())
        .filter(|step| only_step_index.is_none_or(|index| index == step.index))
        .map(|step| (session_id.to_string(), trajectory_turn_id(step), step.index))
        .collect::<BTreeSet<_>>();
    candidate_keys.extend(
        app.active_session_view_state
            .llm_raw_step_indices
            .iter()
            .filter(|(stored_session_id, _, step_index)| {
                stored_session_id == session_id
                    && only_step_index.is_none_or(|index| index == *step_index)
            })
            .cloned(),
    );
    let missing = candidate_keys
        .into_iter()
        .filter(|key| {
            !app.active_session_view_state.step_llm_raw.contains_key(key)
                && !app.trajectory_raw_loading.contains(key)
        })
        .collect::<Vec<_>>();

    let mut tasks = Vec::with_capacity(missing.len());
    let directory = app.known_session_directory(session_id).or_else(|| app.project_path.clone());
    for key in missing {
        app.trajectory_raw_errors.remove(&key);
        app.trajectory_raw_loading.insert(key.clone());
        app.trajectory_raw_attempts.insert(key.clone(), 1);
        tasks.push(raw_load_task(key.0, key.1, key.2, directory.clone(), 1));
    }
    Task::batch(tasks)
}

fn retry_raw_load_task(
    app: &mut App,
    session_id: String,
    turn_id: String,
    step_index: u32,
    attempt: u8,
) -> Task<Message> {
    let key = (session_id.clone(), turn_id.clone(), step_index);
    if app.active_session_id.as_deref() != Some(&session_id) {
        clear_raw_load_state(app, &key);
        return Task::none();
    }
    if !(1..=MAX_RAW_LOAD_ATTEMPTS).contains(&attempt) {
        clear_raw_load_state(app, &key);
        return Task::none();
    }
    if app.trajectory_raw_attempts.get(&key).copied() != Some(attempt) {
        return Task::none();
    }
    if !is_current_completed_missing_raw(app, &session_id, &turn_id, step_index) {
        clear_raw_load_state(app, &key);
        return Task::none();
    }
    let directory = app.known_session_directory(&session_id).or_else(|| app.project_path.clone());
    raw_load_task(session_id, turn_id, step_index, directory, attempt)
}

fn raw_load_task(
    session_id: String,
    turn_id: String,
    step_index: u32,
    directory: Option<String>,
    attempt: u8,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = crate::app::session_gateway::gateway_session_step_llm_raw_async(
                &session_id,
                &turn_id,
                step_index,
                directory.as_deref(),
            )
            .await;
            (session_id, turn_id, step_index, attempt, result)
        },
        |(session_id, turn_id, step_index, attempt, result)| {
            Message::Chat(ChatMessage::TrajectoryRawLoaded {
                session_id,
                turn_id,
                step_index,
                attempt,
                result,
            })
        },
    )
}

fn apply_raw_result(
    app: &mut App,
    session_id: String,
    turn_id: String,
    step_index: u32,
    attempt: u8,
    result: Result<Option<serde_json::Value>, String>,
) -> Task<Message> {
    let key = (session_id.clone(), turn_id.clone(), step_index);
    if app.active_session_id.as_deref() != Some(&session_id) {
        clear_raw_load_state(app, &key);
        return Task::none();
    }
    if !(1..=MAX_RAW_LOAD_ATTEMPTS).contains(&attempt) {
        clear_raw_load_state(app, &key);
        return Task::none();
    }
    if app.trajectory_raw_attempts.get(&key).copied() != Some(attempt) {
        return Task::none();
    }
    if !is_current_completed_missing_raw(app, &session_id, &turn_id, step_index) {
        clear_raw_load_state(app, &key);
        return Task::none();
    }

    match result {
        Ok(Some(raw)) => {
            app.insert_active_session_step_llm_raw(key.clone(), raw);
            clear_raw_load_state(app, &key);
            sync_details_values(app);
            Task::none()
        }
        Ok(None) if attempt < MAX_RAW_LOAD_ATTEMPTS => schedule_raw_retry(app, key, attempt + 1),
        Err(_) if attempt < MAX_RAW_LOAD_ATTEMPTS => schedule_raw_retry(app, key, attempt + 1),
        Ok(None) => {
            clear_raw_load_state(app, &key);
            app.trajectory_raw_errors
                .insert(key, "原始数据自动重试 3 次后仍未就绪，可稍后重试".to_string());
            Task::none()
        }
        Err(error) => {
            clear_raw_load_state(app, &key);
            app.trajectory_raw_errors.insert(key, error);
            Task::none()
        }
    }
}

fn schedule_raw_retry(app: &mut App, key: SessionStepRawKey, attempt: u8) -> Task<Message> {
    let delay = match attempt {
        2 => SECOND_RAW_LOAD_DELAY,
        3 => THIRD_RAW_LOAD_DELAY,
        _ => return Task::none(),
    };
    app.trajectory_raw_loading.insert(key.clone());
    app.trajectory_raw_attempts.insert(key.clone(), attempt);
    app.trajectory_raw_errors.remove(&key);
    crate::app::message::after(
        delay,
        Message::Chat(ChatMessage::TrajectoryRawRetry {
            session_id: key.0,
            turn_id: key.1,
            step_index: key.2,
            attempt,
        }),
    )
}

fn is_current_completed_missing_raw(
    app: &App,
    session_id: &str,
    turn_id: &str,
    step_index: u32,
) -> bool {
    let key = (session_id.to_string(), turn_id.to_string(), step_index);
    app.active_session_id.as_deref() == Some(session_id)
        && !app.active_session_view_state.step_llm_raw.contains_key(&key)
        && (app.active_session_view_state.llm_raw_step_indices.contains(&key)
            || app.active_session_view_state.steps.iter().any(|step| {
                step.index == step_index
                    && trajectory_turn_id(step) == turn_id
                    && step.finished_ms.is_some()
            }))
}

fn trajectory_turn_id(step: &crate::app::models::ChatSessionStep) -> String {
    step.turn_id
        .clone()
        .filter(|turn_id| !turn_id.trim().is_empty())
        .unwrap_or_else(|| format!("legacy-step-{}", step.index))
}

fn clear_raw_load_state(app: &mut App, key: &SessionStepRawKey) {
    app.trajectory_raw_loading.remove(key);
    app.trajectory_raw_attempts.remove(key);
    app.trajectory_raw_errors.remove(key);
}
