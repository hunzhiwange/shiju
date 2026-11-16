//! 聊天面板轨迹模式的瞬时交互与原始载荷补载。

use super::ChatMessage;
use desktop_chat_view::trajectory::update::{self as trajectory_update, Message as UiMessage};
use crate::app::state::ChatPanelMode;
use crate::app::state::SessionStepRawKey;
use crate::app::{App, Message};
use iced::Task;

use desktop_session::message::chat::trajectory::{RawLoadState, RawResultEffect, accepted_snapshot, missing_snapshot_steps};

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
        ChatMessage::TrajectoryRawRetry { session_id, turn_id, step_index, attempt } => {
            retry_raw_load_task(app, session_id, turn_id, step_index, attempt)
        }
        ChatMessage::TrajectoryRawLoaded { session_id, turn_id, step_index, attempt, result } => {
            apply_raw_result(app, session_id, turn_id, step_index, attempt, result)
        }
        ChatMessage::TrajectorySnapshotLoaded { session_id, result } => {
            apply_snapshot_result(app, session_id, result)
        }
        message => {
            let message = match message {
                ChatMessage::TrajectorySearchChanged(search) => UiMessage::TrajectorySearchChanged(search),
                ChatMessage::TrajectoryDurationToggled => UiMessage::TrajectoryDurationToggled,
                ChatMessage::TrajectoryTurnsToggled => UiMessage::TrajectoryTurnsToggled,
                ChatMessage::TrajectoryCallsToggled => UiMessage::TrajectoryCallsToggled,
                ChatMessage::TrajectoryTurnToggled(turn_index) => UiMessage::TrajectoryTurnToggled(turn_index),
                ChatMessage::TrajectoryToolStepToggled(step_index) => UiMessage::TrajectoryToolStepToggled(step_index),
                ChatMessage::TrajectoryRecordSelected(record_id) => UiMessage::TrajectoryRecordSelected(record_id),
                ChatMessage::TrajectoryDetailsTabSelected(tab) => UiMessage::TrajectoryDetailsTabSelected(tab),
                ChatMessage::TrajectoryDetailsClosed => UiMessage::TrajectoryDetailsClosed,
                ChatMessage::TrajectoryDetailsEditorAction(index, action) => UiMessage::TrajectoryDetailsEditorAction(index, action),
                ChatMessage::TrajectoryDetailsContextMenuOpened { index, x, y } => UiMessage::TrajectoryDetailsContextMenuOpened { index, x, y },
                ChatMessage::TrajectoryDetailsContextMenuClosed => UiMessage::TrajectoryDetailsContextMenuClosed,
                ChatMessage::TrajectoryDetailsContextMenuCopy => UiMessage::TrajectoryDetailsContextMenuCopy,
                ChatMessage::TrajectoryDetailsJsonToggled(path) => UiMessage::TrajectoryDetailsJsonToggled(path),
                ChatMessage::TrajectoryDetailsJsonCopied(value) => UiMessage::TrajectoryDetailsJsonCopied(value),
                _ => return Task::none(),
            };
            trajectory_update::update(
                &mut app.trajectory_controls, &mut app.trajectory_details,
                &mut app.trajectory_search_text, app.active_session_id.as_deref(),
                &app.active_session_view_state.trajectory_projection, message,
            ).map_or_else(Task::none, |text| Task::done(Message::CopyCode(text)))
        },
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
    result: Result<Option<gateway_session::SessionUiLoadResult>, String>,
) -> Task<Message> {
    let Some(loaded) = accepted_snapshot(
        app.chat_panel_mode == ChatPanelMode::Trajectory, app.active_session_id.as_deref(),
        &session_id, result,
    ) else { return Task::none(); };
    for step in missing_snapshot_steps(&app.active_session_view_state.steps, loaded.session.steps) {
        app.upsert_active_session_step(step);
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
    trajectory_update::sync_details_values(
        &mut app.trajectory_details, &app.active_session_view_state.trajectory_projection,
    );
}

pub(super) fn refresh_missing_raw_task(
    app: &mut App,
    session_id: &str,
    only_step_index: Option<u32>,
) -> Task<Message> {
    if app.active_session_id.as_deref() != Some(session_id) {
        return Task::none();
    }

    let missing = raw_state(app).missing(session_id, only_step_index);

    let mut tasks = Vec::with_capacity(missing.len());
    let directory = app.known_session_directory(session_id).or_else(|| app.navigation_tabs.project_path.clone());
    for key in missing {
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
    let active_session_id = app.active_session_id.clone();
    if !raw_state(app).accepts(active_session_id.as_deref(), &key, attempt) {
        return Task::none();
    }
    let directory = app.known_session_directory(&session_id).or_else(|| app.navigation_tabs.project_path.clone());
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
    let active_session_id = app.active_session_id.clone();
    let effect = raw_state(app).apply_result(active_session_id.as_deref(), &key, attempt, result);
    match effect {
        RawResultEffect::Insert(raw) => {
            app.insert_active_session_step_llm_raw(key.clone(), raw);
            raw_state(app).clear(&key);
            sync_details_values(app);
            Task::none()
        }
        RawResultEffect::Retry { attempt, delay } => crate::app::message::after(
            delay,
            Message::Chat(ChatMessage::TrajectoryRawRetry {
                session_id: key.0, turn_id: key.1, step_index: key.2, attempt,
            }),
        ),
        RawResultEffect::None => Task::none(),
    }
}

fn raw_state(app: &mut App) -> RawLoadState<'_> {
    RawLoadState {
        steps: &app.active_session_view_state.steps,
        indices: &app.active_session_view_state.llm_raw_step_indices,
        raw: &app.active_session_view_state.step_llm_raw,
        loading: &mut app.trajectory_raw_loading,
        attempts: &mut app.trajectory_raw_attempts,
        errors: &mut app.trajectory_raw_errors,
    }
}
