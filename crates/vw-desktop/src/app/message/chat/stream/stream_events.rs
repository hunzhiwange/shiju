//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。
use desktop_session::message::chat::stream::stream_events::{
    self as domain, TerminalTailCalibration,
};
use desktop_session::message::chat::stream::stream_events::{
    append_stream_delta_to_tail, apply_stream_done_message_ids, leading_guide_count,
    message_ids_tail,
};

use super::{now_ms, session_directory_for_save, start_next};
use crate::app::message::ProjectMessage;
use crate::app::message::chat::scroll_chat_to_bottom_task;
use crate::app::message::chat::session as chat_session;
use crate::app::state::{QuickMessageGitCommitAction, QuickMessageHandoffState};
use crate::app::ui::chat;
use crate::app::{App, Message, models};
use iced::Task;
use session_info;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// 将连续的模型增量合并后再交给 UI，避免每个 token 都触发重排、预热和保存。

pub(super) fn stream_delta_flush_interval(app: &App, session_id: &str) -> Duration {
    desktop_session::message::chat::stream::stream_events::stream_delta_flush_interval(
        app.active_session_id.as_deref(),
        session_id,
    )
}

fn calibrate_stream_tail_at_terminal(
    app: &mut App,
    session_id: &str,
    chat: &mut [models::ChatMessage],
    now: u64,
) -> Option<TerminalTailCalibration> {
    let message_idx = app
        .get_session_runtime(session_id)
        .stream_tail_state
        .message_idx()
        .filter(|idx| {
            chat.get(*idx).is_some_and(|message| message.role == models::ChatRole::Assistant)
        })
        .or_else(|| chat.iter().rposition(|message| message.role == models::ChatRole::Assistant))?;
    let message = &mut chat[message_idx];
    let (thinks, _visible, thinking_open) = chat::split_think(&message.content);
    let estimated_height =
        crate::app::components::chat_panel::message_view::estimate_message_height_rough(
            &message.content,
        );
    desktop_session::message::chat::stream::stream_events::calibrate_terminal_tail(
        &mut app.get_session_runtime_mut(session_id).stream_tail_state,
        message,
        message_idx,
        thinks.len(),
        thinking_open,
        estimated_height,
        now,
    )
}

fn apply_active_terminal_tail_calibration(app: &mut App, calibration: TerminalTailCalibration) {
    app.chat_stream_think_msg_idx = Some(calibration.message_idx);
    app.chat_stream_think_count = calibration.think_count;
    app.chat_stream_think_open_idx = calibration
        .thinking_open
        .then(|| calibration.think_count.saturating_sub(1))
        .filter(|_| calibration.think_count > 0);
    app.apply_stream_tail_estimated_height(calibration.message_idx, calibration.estimated_height);
}

fn completed_handoff_prompt(chat: &[models::ChatMessage]) -> Option<String> {
    chat.iter()
        .rev()
        .find(|message| matches!(message.role, models::ChatRole::Assistant))
        .map(|message| chat::split_think(&message.content).1)
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

fn complete_quick_message_handoff(
    app: &mut App,
    request_id: u64,
    session_id: &str,
    chat: &[models::ChatMessage],
) -> Option<Task<Message>> {
    let current_project = crate::app::components::git_panel::git_repo_path_for_app(app)
        .or_else(|| app.navigation_tabs.project_path.clone());
    let prompt = completed_handoff_prompt(chat);
    match domain::complete_commit_handoff(
        app.quick_message_handoff.as_ref(),
        request_id,
        session_id,
        prompt.as_deref(),
        current_project.as_deref(),
    ) {
        domain::CommitHandoff::Unrelated => None,
        domain::CommitHandoff::Warning(message) => {
            app.quick_message_handoff = None;
            app.git.git_commit_in_progress = false;
            Some(app.show_localized_toast(crate::app::state::ToastKind::Warning, message, vec![]))
        }
        domain::CommitHandoff::Commit { title, summary, action } => {
            app.git.git_commit_type = None;
            app.git.git_commit_scope.clear();
            app.git.git_commit_message = title;
            app.git.git_commit_description = summary;
            app.git.git_commit_description_editor =
                iced::widget::text_editor::Content::with_text(&app.git.git_commit_description);
            app.quick_message_handoff = None;
            app.git.git_commit_in_progress = false;
            let message = match action {
                QuickMessageGitCommitAction::StageAllAndCommit => {
                    crate::app::message::GitMessage::CommitAll
                }
                QuickMessageGitCommitAction::CommitSelected => {
                    crate::app::message::GitMessage::CommitSelected
                }
            };
            Some(Task::done(Message::Git(message)))
        }
    }
}

fn prepare_scroll_for_active_stream_growth(app: &mut App) {
    if app.chat_auto_scroll {
        app.chat_virtualizer.snap_to_tail();
    } else if app.chat_virtualizer.turn_focus_user_index().is_none() {
        app.chat_virtualizer.preserve_current_anchor_around();
    }
    app.sync_chat_scroll_compat_fields();
}


fn finish_active_stream_tail_update(
    app: &mut App,
    session_id: String,
    changed_idx: usize,
    now: u64,
    previous_content_len: Option<usize>,
) -> Task<Message> {
    // 流式追加和工具替换都会改变尾消息；旧哈希不能继续作为视图缓存命中依据。
    // 仅失效变化行，历史消息缓存和编辑器保持可复用。
    app.chat_render_cache.remove(&changed_idx);
    if let Some(visible_text) = app.chat_visible_text_cache.get_mut(changed_idx) {
        *visible_text = None;
    }
    if let Some(copy_hash) = app.chat_copy_hash_cache.get_mut(changed_idx) {
        *copy_hash = None;
    }
    app.session_chat_cache.remove(&session_id);
    app.session_chat_message_id_cache.remove(&session_id);
    prepare_scroll_for_active_stream_growth(app);
    app.sync_task_pet_from_runtime();
    let (estimated_height, stream_generation) = {
        let tail_state = &mut app.get_session_runtime_mut(&session_id).stream_tail_state;
        tail_state.advance_ui_generation();
        (tail_state.estimated_height(), tail_state.begin_ui_preparation())
    };
    app.apply_stream_tail_estimated_height(changed_idx, estimated_height);

    let should_snap_to_bottom = app.chat_virtualizer.should_follow_tail_after_growth();
    if let Some(stream_generation) = stream_generation {
        app.get_session_runtime_mut(&session_id)
            .stream_tail_state
            .finish_ui_preparation(stream_generation);
    }

    app.active_session_view_state.updated_ms = now;
    app.refresh_active_session_tail_message_meta(changed_idx);

    mark_agent_stream_session_dirty(app, &session_id, changed_idx, now, previous_content_len);
    let save_task = schedule_agent_stream_session_save(app, &session_id);

    if should_snap_to_bottom {
        Task::batch(vec![scroll_chat_to_bottom_task(app), save_task])
    } else {
        save_task
    }
}

fn refresh_stream_replacement_projection(app: &mut App, session_id: &str, message_idx: usize) {
    let generation = app
        .session_runtime_states
        .get(session_id)
        .and_then(|runtime| runtime.request_lifecycle.active_agent_request.as_ref())
        .map_or(0, |request| request.id);
    let mut render =
        std::mem::take(&mut app.get_session_runtime_mut(session_id).stream_render_state);
    if let Some(content) = app.chat.get(message_idx).map(|message| message.content.as_str()) {
        render.begin(session_id, generation, message_idx, content);
        let _ = render.replace_from_stable_boundary(session_id, generation, message_idx, content);
    }
    app.get_session_runtime_mut(session_id).stream_render_state = render;
}

pub(super) fn schedule_agent_stream_session_save(app: &mut App, session_id: &str) -> Task<Message> {
    let runtime = app.get_session_runtime_mut(session_id);
    let delay = runtime.stream_save.schedule(runtime.is_plan_run_session);
    if let Some(delay) = delay {
        crate::app::message::after(
            delay,
            Message::Chat(super::ChatMessage::AgentStreamSessionSaveFlush(session_id.into())),
        )
    } else {
        Task::none()
    }
}

pub(super) fn mark_agent_stream_session_dirty(
    app: &mut App,
    session_id: &str,
    index: usize,
    updated_ms: u64,
    previous_content_len: Option<usize>,
) {
    if app.session_runtime_states.get(session_id).is_some_and(|runtime| runtime.is_plan_run_session)
    {
        return;
    }
    let is_active = app.active_session_id.as_deref() == Some(session_id);
    let seq_no = app
        .session_runtime_states
        .get(session_id)
        .map_or(0, |runtime| runtime.stream_save.seq_base)
        + index as u64;
    let message = if is_active {
        app.chat.get(index).map(|source| {
            let (message, _cloned) = app
                .session_runtime_states
                .get_mut(session_id)
                .map(|runtime| runtime.stream_save.snapshot(source, seq_no, previous_content_len))
                .unwrap_or_else(|| (source.clone(), true));
            #[cfg(test)]
            if _cloned {
                crate::app::components::chat_panel::performance::record_chat_message_clone(
                    source.content.len(),
                );
            }
            message
        })
    } else {
        app.cached_chat_messages(session_id).and_then(|messages| messages.get(index).cloned())
    };
    let Some(message) = message else {
        return;
    };
    let message_id = if is_active {
        app.chat_message_ids.get(index).cloned().flatten()
    } else {
        app.cached_chat_message_ids(session_id).and_then(|ids| ids.get(index).cloned()).flatten()
    };
    let base_updated_ms = app
        .sessions
        .iter()
        .find(|session| session.id == session_id)
        .map_or(0, |session| session.time.updated);
    let request_generation = app
        .session_runtime_states
        .get(session_id)
        .and_then(|runtime| runtime.request_lifecycle.active_agent_request.as_ref())
        .map(|request| request.id);
    let runtime = app.get_session_runtime_mut(session_id);
    runtime.stream_save.mark(
        session_ui_types::SessionMessageRecord { seq_no, message_id, message },
        base_updated_ms,
        request_generation,
    );
    // Keep the newest local timestamp in the delta snapshot; the stored base changes only on ack.
    app.active_session_view_state.updated_ms = updated_ms;
}

pub(super) fn flush_agent_stream_session_save(app: &mut App, session_id: String) -> Task<Message> {
    let should_save = {
        let runtime = app.get_session_runtime_mut(&session_id);
        std::mem::take(&mut runtime.stream_save.flush_scheduled)
    };
    if !should_save {
        return Task::none();
    }

    save_latest_agent_stream_session(app, &session_id)
}

fn clear_agent_stream_session_save_flush(app: &mut App, session_id: &str) {
    app.get_session_runtime_mut(session_id).stream_save.flush_scheduled = false;
}

fn save_latest_agent_stream_session(app: &mut App, session_id: &str) -> Task<Message> {
    let directory = session_directory_for_save(app, session_id);
    let runtime = app.get_session_runtime_mut(session_id);
    let Some((generation, revision, delta)) =
        runtime.stream_save.prepare(session_id, runtime.is_plan_run_session, now_ms())
    else {
        return Task::none();
    };
    let updated_ms = delta.updated_ms;
    let saved_session_id = session_id.to_string();
    let result_session_id = session_id.to_string();
    Task::perform(
        async move {
            crate::app::session_gateway::gateway_session_message_delta_async(
                &saved_session_id,
                directory.as_deref(),
                &delta,
            )
            .await
            .map(|_| ())
        },
        move |result| {
            Message::Chat(super::ChatMessage::AgentStreamSessionDeltaSaved {
                session_id: result_session_id.clone(),
                generation,
                revision,
                updated_ms,
                result,
            })
        },
    )
}

pub(super) fn handle_agent_stream_session_delta_saved(
    app: &mut App,
    session_id: String,
    generation: u64,
    revision: u64,
    updated_ms: u64,
    result: Result<(), crate::app::session_gateway::SessionMessageDeltaSaveError>,
) -> Task<Message> {
    use desktop_session::message::chat::stream::save::SaveOutcome;
    let runtime = app.get_session_runtime_mut(&session_id);
    let is_plan_run = runtime.is_plan_run_session;
    let request_generation =
        runtime.request_lifecycle.active_agent_request.as_ref().map(|request| request.id);
    let mut save = std::mem::take(&mut runtime.stream_save);
    let is_active = app.active_session_id.as_deref() == Some(session_id.as_str());
    let base_updated_ms = app
        .sessions
        .iter()
        .find(|session| session.id == session_id)
        .map_or(0, |session| session.time.updated);
    let (outcome, repaired) = save.acknowledge(
        generation,
        revision,
        updated_ms,
        &result,
        base_updated_ms,
        request_generation,
        |index| {
            if is_plan_run {
                return None;
            }
            let message = if is_active {
                app.chat.get(index).map(|source| {
                    #[cfg(test)]
                    crate::app::components::chat_panel::performance::record_chat_message_clone(
                        source.content.len(),
                    );
                    source.clone()
                })
            } else {
                app.cached_chat_messages(&session_id)
                    .and_then(|messages| messages.get(index).cloned())
            }?;
            let message_id = if is_active {
                app.chat_message_ids.get(index).cloned().flatten()
            } else {
                app.cached_chat_message_ids(&session_id)
                    .and_then(|ids| ids.get(index).cloned())
                    .flatten()
            };
            Some((message, message_id))
        },
    );
    app.get_session_runtime_mut(&session_id).stream_save = save;
    if repaired {
        app.active_session_view_state.updated_ms = updated_ms;
    }
    match outcome {
        SaveOutcome::Ignored | SaveOutcome::Settled => Task::none(),
        SaveOutcome::Save => save_latest_agent_stream_session(app, &session_id),
        SaveOutcome::Retry => {
            if let Err(error) = &result {
                tracing::debug!(target: "vw_desktop", session_id = %session_id, error = %error, "retrying stream message save");
            }
            schedule_agent_stream_session_save(app, &session_id)
        }
        SaveOutcome::Stopped => {
            let Err(error) = &result else {
                return Task::none();
            };
            tracing::warn!(target: "vw_desktop", session_id = %session_id, error = %error, "stream message save stopped; keeping unsaved records");
            app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "会话消息保存失败，已停止自动重试，待存消息仍保留在内存中：%s",
                vec![format!("{}", error).into()],
            )
        }
    }
}

pub(super) fn force_save_agent_stream_session(app: &mut App, session_id: &str) -> Task<Message> {
    clear_agent_stream_session_save_flush(app, session_id);
    let now = now_ms();
    if app.active_session_id.as_deref() == Some(session_id) {
        let mut chat = std::mem::take(&mut app.chat);
        let calibration = calibrate_stream_tail_at_terminal(app, session_id, &mut chat, now);
        app.chat = chat;
        if let Some(calibration) = calibration {
            apply_active_terminal_tail_calibration(app, calibration);
        }
    } else if let Some(cached) = app.session_chat_cache.get(session_id).cloned() {
        let mut chat = cached.iter().cloned().collect::<Vec<_>>();
        let calibration = calibrate_stream_tail_at_terminal(app, session_id, &mut chat, now);
        if calibration.is_some() {
            let message_ids = app
                .session_chat_message_id_cache
                .get(session_id)
                .cloned()
                .unwrap_or_else(|| vec![None; chat.len()]);
            app.store_session_chat_snapshot(
                session_id.to_string(),
                crate::app::session::shared_chat_messages(chat),
                message_ids,
            );
        }
    }
    save_latest_agent_stream_session(app, session_id)
}

pub(super) fn queue_agent_stream_delta(app: &mut App, id: u64, delta: String) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };

    queue_agent_session_delta(app, session_id, delta)
}

pub(super) fn queue_agent_tool_delta(app: &mut App, id: u64, delta: String) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    let pending = &mut app.get_session_runtime_mut(&session_id).pending_stream_delta;
    // The flush can combine the preceding text and the tool into one delta.
    desktop_session::message::chat::stream::stream_events::prepare_buffered_tool_line(pending);
    queue_agent_session_delta(app, session_id, delta)
}

pub(super) fn settle_interrupted_compaction(app: &mut App, session_id: &str, content: &str) {
    let is_active = app.active_session_id.as_deref() == Some(session_id);
    let mut chat = if is_active {
        app.chat.clone()
    } else {
        app.session_chat_cache
            .get(session_id)
            .map(|chat| chat.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let Some(index) = desktop_session::message::chat::stream::stream_events::settle_compaction(
        &mut chat,
        &mut app.get_session_runtime_mut(session_id).pending_compaction_status_index,
        crate::app::components::chat_panel::message_view::COMPACTION_STARTED_TEXT,
        content,
    ) else {
        return;
    };
    let changed = [index];
    let ids = if is_active {
        app.chat_message_ids.clone()
    } else {
        app.session_chat_message_id_cache
            .get(session_id)
            .cloned()
            .unwrap_or_else(|| vec![None; chat.len()])
    };
    app.store_session_chat_snapshot(
        session_id.to_string(),
        crate::app::session::shared_chat_messages(chat.clone()),
        ids,
    );
    if is_active {
        app.chat = chat;
        for index in &changed {
            app.chat_render_cache.remove(index);
        }
        app.sync_active_session_from_chat();
    }
    for index in changed {
        mark_agent_stream_session_dirty(app, session_id, index, now_ms(), None);
    }
}

pub(super) fn handle_agent_todo_continuation(app: &mut App, id: u64) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    // 先刷新正文，保证状态条位于上一轮回复与下一轮输出之间。
    let delta_task = flush_agent_stream_delta(app, session_id.clone());
    let is_active = app.active_session_id.as_deref() == Some(session_id.as_str());
    let mut chat = if is_active {
        app.chat.clone()
    } else {
        app.session_chat_cache
            .get(&session_id)
            .map(|messages| messages.iter().cloned().collect())
            .unwrap_or_default()
    };
    let mut message_ids = if is_active {
        app.chat_message_ids.clone()
    } else {
        app.session_chat_message_id_cache.get(&session_id).cloned().unwrap_or_default()
    };
    let changed_idx = desktop_session::message::chat::stream::stream_events::append_system_notice(
        &mut chat,
        &mut message_ids,
        crate::app::components::chat_panel::message_view::TODO_CONTINUATION_TEXT.into(),
    );
    app.get_session_runtime_mut(&session_id).stream_tail_state.clear_tail();
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(chat.clone()),
        message_ids.clone(),
    );
    if is_active {
        app.chat = chat;
        app.chat_message_ids = message_ids;
        app.chat_render_cache.remove(&changed_idx);
        app.sync_active_session_from_chat();
    }
    mark_agent_stream_session_dirty(app, &session_id, changed_idx, now_ms(), None);
    let save_task = schedule_agent_stream_session_save(app, &session_id);
    if is_active {
        Task::batch([delta_task, scroll_chat_to_bottom_task(app), save_task])
    } else {
        Task::batch([delta_task, save_task])
    }
}

pub(super) fn handle_agent_compaction_status(
    app: &mut App,
    id: u64,
    completed: bool,
) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    let is_active = app.active_session_id.as_deref() == Some(session_id.as_str());
    let mut chat = if is_active {
        std::mem::take(&mut app.chat)
    } else {
        app.session_chat_cache
            .get(&session_id)
            .map(|messages| messages.iter().cloned().collect())
            .unwrap_or_default()
    };
    let mut ids = if is_active {
        std::mem::take(&mut app.chat_message_ids)
    } else {
        app.session_chat_message_id_cache
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| vec![None; chat.len()])
    };
    let changed_idx = domain::apply_runtime_compaction(
        app.get_session_runtime_mut(&session_id),
        &mut chat,
        &mut ids,
        completed,
        crate::app::components::chat_panel::message_view::COMPACTION_STARTED_TEXT,
        crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT,
    );
    if is_active {
        app.chat = chat;
        app.chat_message_ids = ids;
        app.chat_render_cache.remove(&changed_idx);
        app.session_chat_cache.remove(&session_id);
        app.session_chat_message_id_cache.remove(&session_id);
        app.sync_active_session_from_chat();
    } else {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(chat),
            ids,
        );
    }
    mark_agent_stream_session_dirty(app, &session_id, changed_idx, now_ms(), None);
    let save = schedule_agent_stream_session_save(app, &session_id);
    if is_active { Task::batch([scroll_chat_to_bottom_task(app), save]) } else { save }
}

pub(super) fn queue_agent_session_delta(
    app: &mut App,
    session_id: String,
    delta: String,
) -> Task<Message> {
    if delta.is_empty() {
        return Task::none();
    }

    use desktop_session::message::chat::stream::stream_events::DeltaEffect;
    let is_active = app.active_session_id.as_deref() == Some(session_id.as_str());
    let interval = stream_delta_flush_interval(app, &session_id);
    let runtime = app.get_session_runtime_mut(&session_id);
    match domain::queue_runtime_delta(runtime, is_active, delta) {
        DeltaEffect::Immediate(delta) => apply_agent_session_delta(app, session_id, delta),
        DeltaEffect::Schedule => crate::app::message::after(
            interval,
            Message::Chat(super::ChatMessage::AgentStreamDeltaFlush(session_id)),
        ),
        DeltaEffect::Buffered => Task::none(),
    }
}

pub(super) fn flush_agent_stream_delta(app: &mut App, session_id: String) -> Task<Message> {
    let delta = {
        let runtime = app.get_session_runtime_mut(&session_id);
        domain::flush_runtime_delta(runtime)
    };
    if delta.is_empty() { Task::none() } else { apply_agent_session_delta(app, session_id, delta) }
}

pub(super) fn flush_agent_stream_delta_for_request(app: &mut App, id: u64) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    flush_agent_stream_delta(app, session_id)
}

/// 模块内可见函数，执行单次流式增量更新；测试和终态处理可绕过合并窗口。
#[cfg(test)]
pub(super) fn handle_agent_stream_delta(app: &mut App, id: u64, delta: String) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };

    handle_agent_session_delta(app, session_id, delta)
}

#[cfg(test)]
pub(super) fn handle_agent_session_delta(
    app: &mut App,
    session_id: String,
    delta: String,
) -> Task<Message> {
    apply_agent_session_delta(app, session_id, delta)
}

fn apply_agent_session_delta(app: &mut App, session_id: String, delta: String) -> Task<Message> {
    // A buffered token has no visible effect until it reaches this point. Keep
    // sidebar metadata and queued-token views cached; tool markers in applied
    // text can change the agent badge, including for background descendants.
    app.session_ui_projection.agent_sessions.get_mut().take();
    let now = now_ms();
    let is_active = app.active_session_id.as_ref() == Some(&session_id);
    let mut tail_state = {
        let runtime = app.get_session_runtime_mut(&session_id);
        std::mem::take(&mut runtime.stream_tail_state)
    };
    if is_active {
        let changed_idx = domain::tail_target(&app.chat);
        let generation = app
            .session_runtime_states
            .get(&session_id)
            .and_then(|runtime| runtime.request_lifecycle.active_agent_request.as_ref())
            .map_or(0, |request| request.id);
        {
            let render = &mut app.get_session_runtime_mut(&session_id).stream_render_state;
            render.begin(&session_id, generation, changed_idx, "");
            let _ = render.append_delta(&session_id, generation, changed_idx, &delta);
        }
        let applied = domain::apply_delta(
            &mut app.chat,
            &mut app.chat_message_ids,
            &mut tail_state,
            delta,
            now,
        );
        let changed_idx = applied.changed_index;
        if applied.boundary_changed {
            app.sync_completed_stream_text(changed_idx);
        }
        app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;
        return finish_active_stream_tail_update(
            app,
            session_id,
            changed_idx,
            now,
            applied.previous_content_len,
        );
    }

    let mut live_chat: Vec<models::ChatMessage> = app
        .session_chat_cache
        .get(&session_id)
        .map(|cached| cached.iter().cloned().collect())
        .unwrap_or_default();
    let mut live_ids = app
        .session_chat_message_id_cache
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| vec![None; live_chat.len()]);

    let changed_idx =
        append_stream_delta_to_tail(&mut live_chat, &mut live_ids, delta, now, &mut tail_state);
    app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;

    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(live_chat),
        live_ids,
    );
    mark_agent_stream_session_dirty(app, &session_id, changed_idx, now, None);
    app.sync_task_pet_from_runtime();
    schedule_agent_stream_session_save(app, &session_id)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_plan_run_session_started(
    app: &mut App,
    parent_request_id: u64,
    parent_session_id: String,
    session_id: String,
    _title: String,
    directory: String,
    query: String,
) -> Task<Message> {
    let parent_session_id = parent_session_id.trim();
    let session_id = session_id.trim();
    let directory = directory.trim();
    if parent_session_id.is_empty()
        || session_id.is_empty()
        || directory.is_empty()
        || app.find_session_by_request_id(parent_request_id).as_deref() != Some(parent_session_id)
    {
        return Task::none();
    }

    if !app.session_chat_cache.contains_key(session_id) {
        let initial_chat = domain::plan_initial_chat(query);
        app.store_session_chat_snapshot(
            session_id.to_string(),
            crate::app::session::shared_chat_messages(initial_chat.clone()),
            vec![None; initial_chat.len()],
        );
    }
    let runtime = app.get_session_runtime_mut(session_id);
    domain::start_plan_runtime(runtime);
    app.sync_task_pet_from_runtime();
    Task::none()
}

pub(super) fn handle_plan_run_session_finished(app: &mut App, session_id: String) -> Task<Message> {
    if session_id.trim().is_empty() {
        return Task::none();
    }
    let _ = flush_agent_stream_delta(app, session_id.clone());
    let save_task = force_save_agent_stream_session(app, &session_id);
    app.get_session_runtime_mut(&session_id).request_lifecycle.is_requesting = false;
    app.sync_task_pet_from_runtime();
    if app.active_session_id.as_ref() == Some(&session_id) {
        app.sync_active_session_from_chat();
    }
    save_task
}

/// 模块内可见函数，执行 handle_agent_workflow_node_update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_workflow_node_update(
    app: &mut App,
    id: u64,
    raw_tool_block: String,
) -> Task<Message> {
    handle_agent_tool_blocks_update(app, id, vec![raw_tool_block])
}

fn handle_agent_tool_blocks_update(
    app: &mut App,
    id: u64,
    raw_tool_blocks: Vec<String>,
) -> Task<Message> {
    if raw_tool_blocks.is_empty() {
        return Task::none();
    }
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };

    // 工具事件必须排在尚未刷新的正文之后，避免合并窗口改变消息顺序。
    let delta_task = flush_agent_stream_delta(app, session_id.clone());

    let now = now_ms();
    let is_active = app.active_session_id.as_ref() == Some(&session_id);
    let mut tail_state = {
        let runtime = app.get_session_runtime_mut(&session_id);
        std::mem::take(&mut runtime.stream_tail_state)
    };
    if is_active {
        let changed_idx = domain::apply_tool_blocks(
            &mut app.chat,
            &mut app.chat_message_ids,
            &mut tail_state,
            raw_tool_blocks,
        );
        app.sync_completed_stream_text(changed_idx);
        refresh_stream_replacement_projection(app, &session_id, changed_idx);
        app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;
        return Task::batch([
            delta_task,
            finish_active_stream_tail_update(app, session_id, changed_idx, now, None),
        ]);
    }

    let mut live_chat: Vec<models::ChatMessage> = app
        .session_chat_cache
        .get(&session_id)
        .map(|cached| cached.iter().cloned().collect())
        .unwrap_or_default();
    let mut live_ids = app
        .session_chat_message_id_cache
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| vec![None; live_chat.len()]);

    domain::apply_tool_blocks(&mut live_chat, &mut live_ids, &mut tail_state, raw_tool_blocks);
    app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;

    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(live_chat),
        live_ids,
    );
    app.sync_task_pet_from_runtime();
    Task::batch([delta_task, schedule_agent_stream_session_save(app, &session_id)])
}

/// 将系统提示词实际读取的协议文件显示为持久工具卡。
pub(super) fn handle_agent_context_loaded(
    app: &mut App,
    id: u64,
    paths: Vec<String>,
) -> Task<Message> {
    let raw_tool_blocks =
        desktop_session::message::chat::stream::stream_events::instruction_read_tool_blocks(paths);
    handle_agent_tool_blocks_update(app, id, raw_tool_blocks)
}

/// 模块内可见函数，执行 handle_agent_stream_done 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_stream_done(
    app: &mut App,
    id: u64,
    usage: models::TokenUsage,
    assistant_message_id: Option<String>,
    parent_message_id: Option<String>,
) -> Task<Message> {
    if let Some(session_id) = app.find_session_by_request_id(id)
        && let Some(draft) = app.get_session_runtime(&session_id).draft
    {
        if app
            .get_session_runtime(&session_id)
            .request_lifecycle
            .active_agent_request
            .as_ref()
            .is_some_and(|request| {
                crate::app::impls::task_plan_run_active(
                    request.workflow_mode_enabled,
                    request.resume_history_only,
                )
            })
        {
            return complete_agent_stream(
                app,
                id,
                usage,
                assistant_message_id,
                parent_message_id,
                None,
            );
        }
        return super::draft::reload_after_response(
            session_id,
            draft.directory,
            id,
            usage,
            assistant_message_id,
            parent_message_id,
        );
    }
    complete_agent_stream(app, id, usage, assistant_message_id, parent_message_id, None)
}

pub(super) fn complete_agent_stream(
    app: &mut App,
    id: u64,
    usage: models::TokenUsage,
    assistant_message_id: Option<String>,
    parent_message_id: Option<String>,
    publication: Option<super::draft::DraftPublication>,
) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    clear_model_recovery(app, &session_id);
    let _ = flush_agent_stream_delta(app, session_id.clone());
    clear_agent_stream_session_save_flush(app, &session_id);
    crate::app::state::clear_pending_guide_handoff(id);
    let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
    let completed_ms = now_ms();
    let terminal = domain::begin_terminal(app.get_session_runtime_mut(&session_id));
    let has_pending_queue = terminal.has_pending_queue;
    let request_started_ms = terminal.request_started_ms;
    let resume_history_only = terminal.resume_history_only;
    let mut persisted_chat: Vec<models::ChatMessage> = if is_active_session {
        app.chat.clone()
    } else if let Some(cached) = app.session_chat_cache.get(&session_id).cloned() {
        cached.iter().cloned().collect::<Vec<models::ChatMessage>>()
    } else {
        tracing::warn!(target: "vw_desktop", session_id = %session_id, "missing bounded chat cache at stream completion");
        Vec::new()
    };
    domain::complete_terminal_messages(&mut persisted_chat, &terminal, completed_ms);
    let terminal_tail =
        calibrate_stream_tail_at_terminal(app, &session_id, &mut persisted_chat, completed_ms);
    let generation = terminal.generation;
    if let Some(content) = persisted_chat
        .iter()
        .rfind(|message| message.role == models::ChatRole::Assistant)
        .map(|message| message.content.as_str())
    {
        let mut render =
            std::mem::take(&mut app.get_session_runtime_mut(&session_id).stream_render_state);
        let _ = render.complete(&session_id, generation, content);
        app.get_session_runtime_mut(&session_id).stream_render_state = render;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let local_voice = crate::app::message::voice::local_chat::model_completed(
        app,
        id,
        &session_id,
        &persisted_chat,
    );
    let handoff_task = complete_quick_message_handoff(app, id, &session_id, &persisted_chat);
    let mut persisted_ids = if is_active_session {
        app.chat_message_ids.clone()
    } else {
        app.session_chat_message_id_cache
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| vec![None; persisted_chat.len()])
    };
    apply_stream_done_message_ids(
        &persisted_chat,
        &mut persisted_ids,
        assistant_message_id.clone(),
        parent_message_id.clone(),
    );
    info!(
        target: "vw_desktop",
        request_id = id,
        session_id = %session_id,
        is_active_session,
        assistant_message_id = ?assistant_message_id,
        parent_message_id = ?parent_message_id,
        message_ids_tail = ?message_ids_tail(&persisted_ids),
        "desktop applied stream message ids"
    );

    if is_active_session {
        app.chat = persisted_chat.clone();
        app.chat_message_ids = persisted_ids.clone();
        app.usage = usage;
        app.last_call_log_path = None;
        if let Some(calibration) = terminal_tail {
            apply_active_terminal_tail_calibration(app, calibration);
        }
    }

    if is_active_session {
        domain::bind_terminal_turn(
            &mut app.active_session_view_state.steps,
            assistant_message_id.as_deref(),
            request_started_ms,
        );
    }
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(persisted_chat),
        persisted_ids,
    );
    let terminal_idx =
        app.cached_chat_messages(&session_id).and_then(|messages| messages.len().checked_sub(1));
    domain::complete_draft_boundary(app.get_session_runtime_mut(&session_id), terminal_idx);
    if let Some(terminal_idx) = terminal_idx {
        mark_agent_stream_session_dirty(app, &session_id, terminal_idx, completed_ms, None);
    }
    let save_task = force_save_agent_stream_session(app, &session_id);
    let is_draft = app.get_session_runtime(&session_id).draft.is_some();
    let refresh_title = publication.map_or_else(Task::none, |publication| {
        super::draft::publish(app, &session_id, publication)
    });
    info!(
        target: "vw_desktop",
        request_id = id,
        session_id = %session_id,
        cached_message_ids_tail = ?app
            .session_chat_message_id_cache
            .get(&session_id)
            .map(|ids| message_ids_tail(ids))
            .unwrap_or_default(),
        "desktop persisted stream message ids"
    );

    {
        let runtime = app.get_session_runtime_mut(&session_id);
        domain::finish_runtime(runtime, !is_active_session);
    }
    app.sync_task_pet_from_runtime();

    if is_active_session {
        app.popover_visibility.send_mode = false;
        app.finalize_stream_text();
        app.sync_active_session_from_chat();
    }

    let effects = domain::completion_effects(
        is_active_session,
        has_pending_queue,
        resume_history_only,
        is_draft,
        handoff_task.is_some(),
        app.task_completion_notification,
    );
    let notification = if effects.notify && crate::app::desktop_notification::SUPPORTED {
        crate::app::desktop_notification::completion_task(app.desktop_locale)
    } else {
        Task::none()
    };
    let next = if effects.start_next { start_next(app, &session_id) } else { Task::none() };
    let refresh_message_ids = if effects.refresh_message_ids {
        app.sessions
            .iter()
            .find(|entry| entry.id == session_id)
            .map(|entry| entry.directory.clone())
            .or_else(|| app.navigation_tabs.project_path.clone())
            .map(|project_path| {
                let session_id = session_id.clone();
                Task::perform(
                    async move {
                        let client = match crate::app::gateway_client() {
                            Ok(client) => client,
                            Err(err) => return Err(err),
                        };
                        domain::reload_terminal_messages(&client, session_id, &project_path).await
                    },
                    |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                )
            })
            .unwrap_or_else(Task::none)
    } else {
        Task::none()
    };
    let session_todos_task = if is_active_session {
        let directory = app
            .sessions
            .iter()
            .find(|entry| entry.id == session_id)
            .map(|entry| entry.directory.clone())
            .or_else(|| app.navigation_tabs.project_path.clone());
        super::todos::load_input_panel_todos_task(session_id.clone(), directory)
    } else {
        Task::none()
    };
    let trajectory_raw_task = if is_active_session {
        super::super::trajectory::refresh_missing_raw_task(app, &session_id, None)
    } else {
        Task::none()
    };

    let summary_session = session_id.clone();
    let summary_directory = app.known_session_directory(&session_id);
    let refresh_summary = Task::perform(
        async move {
            gateway_session::GatewaySessionClient::new(
                crate::app::gateway_client()?.transport().clone(),
            )
            .session_get::<session_info::Info>(&summary_session, summary_directory.as_deref())
            .await
        },
        |result| Message::Chat(super::ChatMessage::SessionSummaryReloaded(result)),
    );
    let completion = if is_active_session && app.chat_auto_scroll {
        scroll_chat_to_bottom_task(app)
    } else {
        Task::none()
    }
    .chain(session_todos_task)
    .chain(refresh_summary)
    .chain(refresh_message_ids)
    .chain(save_task)
    .chain(trajectory_raw_task)
    .chain(next)
    .chain(handoff_task.unwrap_or_else(Task::none));
    Task::batch([
        completion,
        notification,
        refresh_title,
        #[cfg(not(target_arch = "wasm32"))]
        local_voice,
    ])
}

/// 模块内可见函数，执行 handle_agent_post_tool_round 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_post_tool_round(
    app: &mut App,
    id: u64,
    session_id: String,
    step_index: u32,
) -> Task<Message> {
    let Some(found_session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    if found_session_id != session_id || app.active_session_id.as_ref() != Some(&session_id) {
        return Task::none();
    }

    let _ = flush_agent_stream_delta(app, session_id.clone());
    clear_agent_stream_session_save_flush(app, &session_id);

    let runtime = app.get_session_runtime(&session_id);
    if runtime.draft.is_some() {
        return Task::none();
    }
    let Some(active_request) =
        runtime.request_lifecycle.active_agent_request.as_ref().map(Arc::clone)
    else {
        return Task::none();
    };
    let insert_idx = leading_guide_count(&runtime.request_lifecycle.queue);
    let completed_ms = now_ms();
    let elapsed = {
        let runtime = app.get_session_runtime_mut(&session_id);
        crate::app::message::chat::processing_timing::pause_runtime_processing_timers(runtime)
    };
    crate::app::message::chat::processing_timing::complete_tail_assistant_processing(
        &mut app.chat,
        completed_ms,
        elapsed.processing_ms,
        elapsed.llm_ms,
    );
    app.finalize_stream_text();
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );
    if let Some(tail_idx) = app.chat.len().checked_sub(1) {
        mark_agent_stream_session_dirty(app, &session_id, tail_idx, completed_ms, None);
    }
    let save_task = force_save_agent_stream_session(app, &session_id);

    let resume_history = app.chat.clone();
    let continuation = desktop_session::message::chat::stream::stream_events::continuation_item(
        &active_request,
        resume_history,
        now_ms(),
    );
    let next_item = {
        let runtime = app.get_session_runtime_mut(&session_id);
        domain::yield_tool_round(runtime, continuation, insert_idx)
    };

    app.sync_task_pet_from_runtime();
    app.popover_visibility.send_mode = false;
    app.sync_active_session_from_chat();
    info!(
        target: "vw_desktop",
        request_id = id,
        session_id = %session_id,
        step_index,
        queued_after_handoff = app.get_session_runtime(&session_id).request_lifecycle.queue.len(),
        "desktop yielded active request after tool round to prioritize guided queue item"
    );

    save_task.chain(chat_session::start(app, next_item, true))
}

/// 模块内可见函数，执行 handle_agent_stream_error 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_stream_error(app: &mut App, id: u64, err: String) -> Task<Message> {
    if matches!(
        app.quick_message_handoff,
        Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
            request_id: pending_id,
            ..
        }) if pending_id == id
    ) {
        if matches!(
            app.quick_message_handoff,
            Some(QuickMessageHandoffState::WaitingForGitCommitMessage { .. })
        ) {
            app.git.git_commit_in_progress = false;
        }
        app.quick_message_handoff = None;
    }
    let display_err = domain::display_stream_error(err);

    let Some(session_id) = app.find_session_by_request_id(id) else {
        crate::app::state::clear_pending_guide_handoff(id);
        if let Some(active_id) = app.active_session_id.clone() {
            let runtime = app.get_session_runtime_mut(&active_id);
            // 迟到的旧请求错误不得结束正在运行的新请求。
            if !domain::settle_unmatched_error(runtime) {
                return Task::none();
            }
            app.sync_task_pet_from_runtime();
        }
        return app.show_error_toast(display_err);
    };
    let recovery = clear_model_recovery(app, &session_id);
    let display_err = if let Some((request, event)) = recovery
        && request == id
        && event.phase == gateway_chat_stream::GatewayModelRecoveryPhase::Stopped
    {
        crate::app::components::chat_panel::message_view::model_recovery_text(&event)
    } else {
        display_err
    };
    settle_interrupted_compaction(
        app,
        &session_id,
        crate::app::components::chat_panel::message_view::COMPACTION_FAILED_TEXT,
    );
    let _ = flush_agent_stream_delta(app, session_id.clone());
    clear_agent_stream_session_save_flush(app, &session_id);
    crate::app::state::clear_pending_guide_handoff(id);
    let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
    let completed_ms = now_ms();

    let terminal = domain::begin_terminal(app.get_session_runtime_mut(&session_id));
    domain::finish_runtime(app.get_session_runtime_mut(&session_id), false);
    app.sync_task_pet_from_runtime();
    if is_active_session {
        app.popover_visibility.send_mode = false;
    }

    let toast_task = app.show_error_toast(display_err.clone());

    let mut persisted_chat = if is_active_session {
        app.chat.clone()
    } else if let Some(cached) = app.session_chat_cache.get(&session_id).cloned() {
        cached.iter().cloned().collect::<Vec<models::ChatMessage>>()
    } else {
        tracing::warn!(target: "vw_desktop", session_id = %session_id, "missing bounded chat cache at stream failure; preserving stored history");
        // A cache miss is not an empty transcript. Never save an error-only replacement.
        return toast_task.chain(start_next(app, &session_id));
    };
    let mut persisted_ids = if is_active_session {
        app.chat_message_ids.clone()
    } else {
        app.session_chat_message_id_cache
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| vec![None; persisted_chat.len()])
    };
    domain::complete_terminal_messages(&mut persisted_chat, &terminal, completed_ms);
    let terminal_tail =
        calibrate_stream_tail_at_terminal(app, &session_id, &mut persisted_chat, completed_ms);
    domain::append_system_notice(&mut persisted_chat, &mut persisted_ids, display_err);
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(persisted_chat.clone()),
        persisted_ids.clone(),
    );

    if is_active_session {
        app.chat = persisted_chat.clone();
        app.chat_message_ids = persisted_ids.clone();
        if let Some(calibration) = terminal_tail {
            apply_active_terminal_tail_calibration(app, calibration);
        }
        app.finalize_stream_text();
        app.sync_active_session_from_chat();
    }

    let message_count = if is_active_session {
        app.chat.len()
    } else {
        app.cached_chat_messages(&session_id).map_or(0, |messages| messages.len())
    };
    for index in message_count.saturating_sub(2)..message_count {
        mark_agent_stream_session_dirty(app, &session_id, index, completed_ms, None);
    }
    let save_task = force_save_agent_stream_session(app, &session_id);

    let next = start_next(app, &session_id);

    if is_active_session && app.chat_auto_scroll {
        toast_task.chain(scroll_chat_to_bottom_task(app)).chain(save_task).chain(next)
    } else {
        toast_task.chain(save_task).chain(next)
    }
}
#[cfg(test)]
#[path = "stream_events_tests.rs"]
mod stream_events_tests;

pub(super) fn clear_model_recovery(
    app: &mut App,
    session_id: &str,
) -> Option<(u64, gateway_chat_stream::GatewayModelRecoveryEvent)> {
    let recovery = app.get_session_runtime_mut(session_id).model_recovery.take();
    if let Some((_, event)) = &recovery
        && app.active_session_id.as_deref() == Some(session_id)
        && app.active_toast.as_ref().is_some_and(|toast| {
            toast.message
                == crate::app::components::chat_panel::message_view::model_recovery_text(event)
        })
    {
        app.active_toast = None;
    }
    recovery
}

pub(super) fn handle_agent_model_recovery(
    app: &mut App,
    id: u64,
    event: gateway_chat_stream::GatewayModelRecoveryEvent,
) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    if !domain::model_recovery_matches(
        app.get_session_runtime(&session_id),
        id,
        &session_id,
        &event,
    ) {
        return Task::none();
    }
    clear_model_recovery(app, &session_id);
    let text = crate::app::components::chat_panel::message_view::model_recovery_text(&event);
    let phase = event.phase;
    app.get_session_runtime_mut(&session_id).model_recovery = Some((id, event));
    if app.active_session_id.as_deref() != Some(session_id.as_str()) {
        return Task::none();
    }
    match phase {
        gateway_chat_stream::GatewayModelRecoveryPhase::Waiting => {
            app.show_toast(crate::app::state::ToastKind::Info, text)
        }
        gateway_chat_stream::GatewayModelRecoveryPhase::Stopped => app.show_error_toast(text),
        gateway_chat_stream::GatewayModelRecoveryPhase::Recovered
        | gateway_chat_stream::GatewayModelRecoveryPhase::Cancelled => Task::none(),
    }
}
