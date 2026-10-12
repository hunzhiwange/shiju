//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::{
    load_session_or_default, now_ms, save_session_task, session_directory_for_save, start_next,
};
use crate::app::message::ProjectMessage;
use crate::app::message::chat::scroll_chat_to_bottom_task;
use crate::app::message::chat::session as chat_session;
use crate::app::state::{
    ChatSendBehavior, QuickMessageGitCommitAction, QuickMessageHandoffState, StreamTailState,
};
use crate::app::ui::chat;
use crate::app::{App, Message, QueueItem, models};
use iced::Task;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use vw_shared::message::types as message;
use vw_shared::session::info as session_info;

/// 将连续的模型增量合并后再交给 UI，避免每个 token 都触发重排、预热和保存。
pub(super) const ACTIVE_STREAM_DELTA_FLUSH_INTERVAL: Duration = Duration::from_millis(50);
pub(super) const BACKGROUND_STREAM_DELTA_FLUSH_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const STREAM_SESSION_SAVE_FLUSH_INTERVAL: Duration = Duration::from_secs(1);

pub(super) fn stream_delta_flush_interval(app: &App, session_id: &str) -> Duration {
    if app.active_session_id.as_deref() == Some(session_id) {
        ACTIVE_STREAM_DELTA_FLUSH_INTERVAL
    } else {
        BACKGROUND_STREAM_DELTA_FLUSH_INTERVAL
    }
}

fn update_think_timing_from_delta(
    message: &mut models::ChatMessage,
    tail_state: &StreamTailState,
    now: u64,
) {
    let think_count = tail_state.think_count();

    if message.think_timing.len() < think_count {
        for _ in message.think_timing.len()..think_count {
            message.think_timing.push(models::ThinkTiming {
                start_ms: now,
                end_ms: None,
                last_update_ms: now,
            });
        }
    }

    if think_count == 0 {
        return;
    }

    if tail_state.thinking_open() {
        let last_index = message.think_timing.len().saturating_sub(1);
        for (idx, entry) in message.think_timing.iter_mut().enumerate() {
            if idx == last_index {
                if entry.end_ms.is_none() {
                    entry.last_update_ms = now;
                }
            } else if entry.end_ms.is_none() {
                entry.end_ms = Some(now);
                entry.last_update_ms = now;
            }
        }
    } else {
        for entry in message.think_timing.iter_mut() {
            if entry.end_ms.is_none() {
                entry.end_ms = Some(now);
                entry.last_update_ms = now;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TerminalTailCalibration {
    message_idx: usize,
    think_count: usize,
    thinking_open: bool,
    estimated_height: f32,
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
    message.think_timing.truncate(thinks.len());
    let calibrated_state = {
        let state = &mut app.get_session_runtime_mut(session_id).stream_tail_state;
        state.calibrate(
            message_idx,
            message.content.len(),
            thinks.len(),
            thinking_open,
            estimated_height,
        );
        state.advance_ui_generation();
        state.clone()
    };
    update_think_timing_from_delta(message, &calibrated_state, now);
    app.get_session_runtime_mut(session_id).stream_tail_state.clear_tail();
    Some(TerminalTailCalibration {
        message_idx,
        think_count: thinks.len(),
        thinking_open,
        estimated_height,
    })
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

fn apply_stream_done_message_ids(
    chat: &[models::ChatMessage],
    message_ids: &mut Vec<Option<String>>,
    assistant_message_id: Option<String>,
    parent_message_id: Option<String>,
) {
    if message_ids.len() < chat.len() {
        message_ids.resize(chat.len(), None);
    }

    let assistant_idx =
        chat.iter().rposition(|message| matches!(message.role, models::ChatRole::Assistant));
    if let Some(idx) = assistant_idx
        && let Some(message_id) = assistant_message_id
    {
        message_ids[idx] = Some(message_id);
    }

    let user_search_end = assistant_idx.unwrap_or(chat.len());
    let user_idx = chat[..user_search_end]
        .iter()
        .rposition(|message| matches!(message.role, models::ChatRole::User));
    if let Some(idx) = user_idx
        && let Some(message_id) = parent_message_id
    {
        message_ids[idx] = Some(message_id);
    }
}

fn message_ids_tail(message_ids: &[Option<String>]) -> Vec<Option<String>> {
    let tail_len = message_ids.len().min(4);
    message_ids[message_ids.len().saturating_sub(tail_len)..].to_vec()
}

fn completed_handoff_prompt(chat: &[models::ChatMessage]) -> Option<String> {
    chat.iter()
        .rev()
        .find(|message| matches!(message.role, models::ChatRole::Assistant))
        .map(|message| chat::split_think(&message.content).1)
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

#[derive(Deserialize)]
struct GeneratedGitCommitMessage {
    #[serde(alias = "subject")]
    title: String,
    #[serde(alias = "description", alias = "body")]
    summary: String,
}

fn parse_generated_git_commit_message(content: &str) -> Option<GeneratedGitCommitMessage> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end < start {
        return None;
    }
    let generated: GeneratedGitCommitMessage = serde_json::from_str(&content[start..=end]).ok()?;
    if generated.title.trim().is_empty() || generated.summary.trim().is_empty() {
        return None;
    }
    Some(generated)
}

fn complete_quick_message_handoff(
    app: &mut App,
    request_id: u64,
    session_id: &str,
    chat: &[models::ChatMessage],
) -> Option<Task<Message>> {
    match app.quick_message_handoff.clone()? {
        QuickMessageHandoffState::WaitingForGitCommitMessage {
            request_id: pending_request_id,
            source_session_id,
            project_path,
            action,
        } => {
            if pending_request_id != request_id || source_session_id != session_id {
                return None;
            }
            let generated = completed_handoff_prompt(chat)
                .as_deref()
                .and_then(parse_generated_git_commit_message);
            let Some(generated) = generated else {
                app.quick_message_handoff = None;
                app.git_commit_in_progress = false;
                return Some(app.show_warning_toast("AI 未生成有效的提交标题和摘要，未执行提交"));
            };
            let current_project_path =
                crate::app::components::git_panel::git_repo_path_for_app(app)
                    .or_else(|| app.project_path.clone());
            if current_project_path.as_deref() != Some(project_path.as_str()) {
                app.quick_message_handoff = None;
                app.git_commit_in_progress = false;
                return Some(app.show_warning_toast("Git 工作树已切换，未执行提交"));
            }

            app.git_commit_type = None;
            app.git_commit_scope.clear();
            app.git_commit_message = generated.title.trim().to_string();
            app.git_commit_description = generated.summary.trim().to_string();
            app.git_commit_description_editor =
                iced::widget::text_editor::Content::with_text(&app.git_commit_description);
            app.quick_message_handoff = None;
            app.git_commit_in_progress = false;
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

fn parse_tool_block_len(s: &str) -> Option<usize> {
    if !s.starts_with("tool ") {
        return None;
    }

    let line_end = s.find('\n')?;
    let mut idx = line_end + 1;
    let mut buf = String::new();
    for _ in 0..64 {
        if idx >= s.len() {
            break;
        }
        let next_end = s[idx..].find('\n').map(|offset| idx + offset).unwrap_or(s.len());
        let line = &s[idx..next_end];
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);
        if serde_json::from_str::<serde_json::Value>(buf.trim()).is_ok() {
            return Some(if next_end < s.len() { next_end + 1 } else { next_end });
        }
        if next_end >= s.len() {
            break;
        }
        idx = next_end + 1;
    }
    None
}

fn tool_call_id_from_raw_block(raw: &str) -> Option<String> {
    let (_, rest) = raw.split_once('\n')?;
    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    value.get("tool_call_id").and_then(serde_json::Value::as_str).map(ToOwned::to_owned)
}

fn tool_block_start_allowed(content: &str, start: usize) -> bool {
    if start == 0 || content[..start].ends_with('\n') {
        return true;
    }
    let prefix = content[..start].trim_end_matches([' ', '\t']);
    prefix.ends_with(':') || prefix.ends_with('：')
}

fn upsert_tool_block_by_call_id(content: &mut String, raw_tool_block: &str) {
    let Some(call_id) = tool_call_id_from_raw_block(raw_tool_block) else {
        append_tool_block(content, raw_tool_block);
        return;
    };

    let mut search_from = 0usize;
    while let Some(offset) = content[search_from..].find("tool ") {
        let start = search_from + offset;
        if !tool_block_start_allowed(content, start) {
            search_from = start + "tool ".len();
            continue;
        }
        let Some(block_len) = parse_tool_block_len(&content[start..]) else {
            search_from = start + "tool ".len();
            continue;
        };
        let end = start + block_len;
        if tool_call_id_from_raw_block(&content[start..end]).as_deref() == Some(call_id.as_str()) {
            content.replace_range(start..end, raw_tool_block);
            return;
        }
        search_from = end;
    }

    append_tool_block(content, raw_tool_block);
}

fn append_tool_block(content: &mut String, raw_tool_block: &str) {
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(raw_tool_block.trim_end());
    content.push('\n');
}

fn leading_guide_count(queue: &[QueueItem]) -> usize {
    queue.iter().take_while(|item| item.send_behavior == ChatSendBehavior::Guide).count()
}

fn continuation_label_from_history(history: &[models::ChatMessage]) -> String {
    history
        .iter()
        .rev()
        .find(|message| matches!(message.role, models::ChatRole::User))
        .map(|message| message.content.clone())
        .unwrap_or_default()
}

fn prepare_scroll_for_active_stream_growth(app: &mut App) {
    if app.chat_auto_scroll {
        app.chat_virtualizer.snap_to_tail();
    } else if app.chat_virtualizer.turn_focus_user_index().is_none() {
        app.chat_virtualizer.preserve_current_anchor_around();
    }
    app.sync_chat_virtualizer_compat_fields();
}

fn append_stream_delta_to_tail(
    chat: &mut Vec<models::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    delta: String,
    now: u64,
    tail_state: &mut StreamTailState,
) -> usize {
    if let Some(last_idx) = chat.len().checked_sub(1)
        && chat[last_idx].role == models::ChatRole::Assistant
    {
        let last = &mut chat[last_idx];
        tail_state.begin_message(last_idx);
        tail_state.apply_delta(&delta);
        update_think_timing_from_delta(last, tail_state, now);
        if !last.content.is_empty() && !last.content.ends_with('\n') && delta.starts_with("tool ") {
            last.content.push('\n');
        }
        last.content.push_str(&delta);
        return last_idx;
    }

    if message_ids.len() < chat.len() {
        message_ids.resize(chat.len(), None);
    }
    tail_state.begin_message(chat.len());
    tail_state.apply_delta(&delta);
    chat.push(models::ChatMessage {
        role: models::ChatRole::Assistant,
        content: delta,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    message_ids.push(None);
    let last_idx = chat.len() - 1;
    update_think_timing_from_delta(&mut chat[last_idx], tail_state, now);
    last_idx
}

fn upsert_workflow_tool_to_tail(
    chat: &mut Vec<models::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    raw_tool_block: String,
) -> usize {
    if let Some(last_idx) = chat.len().checked_sub(1)
        && chat[last_idx].role == models::ChatRole::Assistant
    {
        upsert_tool_block_by_call_id(&mut chat[last_idx].content, &raw_tool_block);
        return last_idx;
    }

    if message_ids.len() < chat.len() {
        message_ids.resize(chat.len(), None);
    }
    let mut content = String::new();
    append_tool_block(&mut content, &raw_tool_block);
    chat.push(models::ChatMessage {
        role: models::ChatRole::Assistant,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    message_ids.push(None);
    chat.len() - 1
}

fn finish_active_stream_tail_update(
    app: &mut App,
    session_id: String,
    changed_idx: usize,
    now: u64,
) -> Task<Message> {
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

    let mut tail_prewarm_task = Task::none();
    let should_snap_to_bottom = app.chat_virtualizer.should_follow_tail_after_growth();
    if let Some(stream_generation) = stream_generation
        && let Some(message) = app.chat.get(changed_idx).cloned()
    {
        tail_prewarm_task = crate::app::message::project::prepare_stream_message_ui_task(
            session_id.clone(),
            message,
            changed_idx,
            stream_generation,
            app.dialogue_flow_show_reasoning_summary,
        );
    }

    app.active_session_view_state.updated_ms = now;
    app.refresh_active_session_tail_message_meta(changed_idx);

    let save_task = schedule_agent_stream_session_save(app, &session_id);

    if should_snap_to_bottom {
        Task::batch(vec![scroll_chat_to_bottom_task(app), tail_prewarm_task, save_task])
    } else {
        Task::batch(vec![tail_prewarm_task, save_task])
    }
}

fn schedule_agent_stream_session_save(app: &mut App, session_id: &str) -> Task<Message> {
    let schedule_flush = {
        let runtime = app.get_session_runtime_mut(session_id);
        if runtime.stream_save_flush_scheduled {
            false
        } else {
            runtime.stream_save_flush_scheduled = true;
            true
        }
    };

    if schedule_flush {
        crate::app::message::after(
            STREAM_SESSION_SAVE_FLUSH_INTERVAL,
            Message::Chat(super::ChatMessage::AgentStreamSessionSaveFlush(session_id.to_string())),
        )
    } else {
        Task::none()
    }
}

pub(super) fn flush_agent_stream_session_save(app: &mut App, session_id: String) -> Task<Message> {
    let should_save = {
        let runtime = app.get_session_runtime_mut(&session_id);
        std::mem::take(&mut runtime.stream_save_flush_scheduled)
    };
    if !should_save {
        return Task::none();
    }

    save_latest_agent_stream_session(app, &session_id)
}

fn clear_agent_stream_session_save_flush(app: &mut App, session_id: &str) {
    app.get_session_runtime_mut(session_id).stream_save_flush_scheduled = false;
}

fn save_latest_agent_stream_session(app: &App, session_id: &str) -> Task<Message> {
    let mut session = load_session_or_default(app, session_id.to_string());
    session.updated_ms = now_ms();
    save_session_task(session, session_directory_for_save(app, session_id))
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

fn upsert_compaction_status(
    chat: &mut Vec<models::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    completed: bool,
) -> usize {
    let content = if completed {
        crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT
    } else {
        crate::app::components::chat_panel::message_view::COMPACTION_STARTED_TEXT
    };
    if completed
        && let Some(idx) = chat.iter().rposition(|message| {
            message.role == models::ChatRole::System
                && message.content
                    == crate::app::components::chat_panel::message_view::COMPACTION_STARTED_TEXT
        })
    {
        chat[idx].content = content.to_string();
        return idx;
    }

    if message_ids.len() < chat.len() {
        message_ids.resize(chat.len(), None);
    }
    chat.push(models::ChatMessage {
        role: models::ChatRole::System,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    message_ids.push(None);
    chat.len() - 1
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
    if is_active {
        let changed_idx =
            upsert_compaction_status(&mut app.chat, &mut app.chat_message_ids, completed);
        app.chat_render_cache.remove(&changed_idx);
        app.session_chat_cache.remove(&session_id);
        app.session_chat_message_id_cache.remove(&session_id);
        app.sync_active_session_from_chat();
        return Task::batch([
            scroll_chat_to_bottom_task(app),
            schedule_agent_stream_session_save(app, &session_id),
        ]);
    }

    let mut chat: Vec<models::ChatMessage> = app
        .session_chat_cache
        .get(&session_id)
        .map(|messages| messages.iter().cloned().collect())
        .unwrap_or_default();
    let mut message_ids = app
        .session_chat_message_id_cache
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| vec![None; chat.len()]);
    upsert_compaction_status(&mut chat, &mut message_ids, completed);
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(chat),
        message_ids,
    );
    schedule_agent_stream_session_save(app, &session_id)
}

pub(super) fn queue_agent_session_delta(
    app: &mut App,
    session_id: String,
    delta: String,
) -> Task<Message> {
    if delta.is_empty() {
        return Task::none();
    }

    let is_active = app.active_session_id.as_deref() == Some(session_id.as_str());
    let immediate_delta = if is_active {
        let runtime = app.get_session_runtime_mut(&session_id);
        if runtime.stream_tail_state.message_idx().is_none() {
            runtime.stream_delta_flush_scheduled = false;
            let mut immediate_delta = std::mem::take(&mut runtime.pending_stream_delta);
            immediate_delta.push_str(&delta);
            Some(immediate_delta)
        } else {
            None
        }
    } else {
        None
    };
    if let Some(immediate_delta) = immediate_delta {
        return apply_agent_session_delta(app, session_id, immediate_delta);
    }

    let flush_interval = stream_delta_flush_interval(app, &session_id);
    let schedule_flush = {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.pending_stream_delta.push_str(&delta);
        if runtime.stream_delta_flush_scheduled {
            false
        } else {
            runtime.stream_delta_flush_scheduled = true;
            true
        }
    };

    if schedule_flush {
        crate::app::message::after(
            flush_interval,
            Message::Chat(super::ChatMessage::AgentStreamDeltaFlush(session_id)),
        )
    } else {
        Task::none()
    }
}

pub(super) fn flush_agent_stream_delta(app: &mut App, session_id: String) -> Task<Message> {
    let delta = {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.stream_delta_flush_scheduled = false;
        std::mem::take(&mut runtime.pending_stream_delta)
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
    let now = now_ms();
    let is_active = app.active_session_id.as_ref() == Some(&session_id);
    let mut tail_state = {
        let runtime = app.get_session_runtime_mut(&session_id);
        std::mem::take(&mut runtime.stream_tail_state)
    };
    if is_active {
        let changed_idx = append_stream_delta_to_tail(
            &mut app.chat,
            &mut app.chat_message_ids,
            delta,
            now,
            &mut tail_state,
        );
        app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;
        return finish_active_stream_tail_update(app, session_id, changed_idx, now);
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

    append_stream_delta_to_tail(&mut live_chat, &mut live_ids, delta, now, &mut tail_state);
    app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;

    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(live_chat),
        live_ids,
    );
    app.sync_task_pet_from_runtime();
    schedule_agent_stream_session_save(app, &session_id)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_plan_run_session_started(
    app: &mut App,
    parent_request_id: u64,
    parent_session_id: String,
    session_id: String,
    title: String,
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

    let parent_info = app
        .sessions
        .iter()
        .find(|session| session.id == parent_session_id)
        .or_else(|| {
            app.project_sessions
                .values()
                .flat_map(|sessions| sessions.iter())
                .find(|session| session.id == parent_session_id)
        })
        .cloned();
    let Some(mut child_info) = parent_info else {
        return Task::none();
    };

    let now = now_ms();
    child_info.id = session_id.to_string();
    child_info.slug = session_info::Info::plan_run_slug(session_id);
    child_info.directory = directory.to_string();
    child_info.parent_id = Some(parent_session_id.to_string());
    child_info.title = if title.trim().is_empty() {
        format!("子会话 · {session_id}")
    } else {
        title.trim().to_string()
    };
    child_info.time.created = now;
    child_info.time.updated = now;
    child_info.time.compacting = None;
    child_info.time.archived = None;

    if let Some(existing) = app.sessions.iter_mut().find(|session| session.id == session_id) {
        *existing = child_info.clone();
    } else {
        app.sessions.insert(0, child_info.clone());
    }
    let project_sessions = app.project_sessions.entry(directory.to_string()).or_default();
    if let Some(existing) = project_sessions.iter_mut().find(|session| session.id == session_id) {
        *existing = child_info;
    } else {
        project_sessions.insert(0, child_info);
    }

    if !app.session_chat_cache.contains_key(session_id) {
        let initial_chat = if query.trim().is_empty() {
            Vec::new()
        } else {
            vec![models::ChatMessage {
                role: models::ChatRole::User,
                content: query,
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            }]
        };
        app.store_session_chat_snapshot(
            session_id.to_string(),
            crate::app::session::shared_chat_messages(initial_chat.clone()),
            vec![None; initial_chat.len()],
        );
    }
    app.get_session_runtime_mut(session_id).is_requesting = true;
    app.sync_task_pet_from_runtime();
    Task::none()
}

pub(super) fn handle_plan_run_session_finished(app: &mut App, session_id: String) -> Task<Message> {
    if session_id.trim().is_empty() {
        return Task::none();
    }
    let _ = flush_agent_stream_delta(app, session_id.clone());
    let save_task = force_save_agent_stream_session(app, &session_id);
    app.get_session_runtime_mut(&session_id).is_requesting = false;
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
        let mut changed_idx = app.chat.len().saturating_sub(1);
        for raw_tool_block in raw_tool_blocks {
            let target_idx = app
                .chat
                .last()
                .is_some_and(|message| message.role == models::ChatRole::Assistant)
                .then_some(app.chat.len().saturating_sub(1))
                .unwrap_or(app.chat.len());
            tail_state.begin_message(target_idx);
            tail_state.apply_delta(&raw_tool_block);
            changed_idx = upsert_workflow_tool_to_tail(
                &mut app.chat,
                &mut app.chat_message_ids,
                raw_tool_block,
            );
        }
        app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;
        return Task::batch([
            delta_task,
            finish_active_stream_tail_update(app, session_id, changed_idx, now),
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

    for raw_tool_block in raw_tool_blocks {
        let target_idx = live_chat
            .last()
            .is_some_and(|message| message.role == models::ChatRole::Assistant)
            .then_some(live_chat.len().saturating_sub(1))
            .unwrap_or(live_chat.len());
        tail_state.begin_message(target_idx);
        tail_state.apply_delta(&raw_tool_block);
        upsert_workflow_tool_to_tail(&mut live_chat, &mut live_ids, raw_tool_block);
    }
    app.get_session_runtime_mut(&session_id).stream_tail_state = tail_state;

    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(live_chat),
        live_ids,
    );
    app.sync_task_pet_from_runtime();
    Task::batch([delta_task, schedule_agent_stream_session_save(app, &session_id)])
}

fn instruction_read_tool_block(path: &str) -> String {
    let input = serde_json::json!({ "filePath": path }).to_string();
    let payload = serde_json::json!({
        "status": "completed",
        "input": input,
        "tool_call_id": format!("system-instruction:{path}"),
        "metadata": {
            "system_instruction": true
        }
    });
    format!("tool read\n{payload}")
}

/// 将系统提示词实际读取的协议文件显示为持久工具卡。
pub(super) fn handle_agent_context_loaded(
    app: &mut App,
    id: u64,
    paths: Vec<String>,
) -> Task<Message> {
    let mut seen = std::collections::HashSet::new();
    let raw_tool_blocks = paths
        .into_iter()
        .filter(|path| !path.trim().is_empty())
        .filter(|path| seen.insert(path.clone()))
        .map(|path| instruction_read_tool_block(&path))
        .collect::<Vec<_>>();
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
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    let _ = flush_agent_stream_delta(app, session_id.clone());
    clear_agent_stream_session_save_flush(app, &session_id);
    crate::app::state::clear_pending_guide_handoff(id);
    let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
    let completed_ms = now_ms();
    let has_pending_queue = !app.get_session_runtime(&session_id).queue.is_empty();
    let request_started_ms = app.get_session_runtime(&session_id).request_started_ms;
    let resume_history_only = app
        .get_session_runtime(&session_id)
        .active_agent_request
        .as_ref()
        .map(|request| request.resume_history_only)
        .unwrap_or(false);
    let elapsed = {
        let runtime = app.get_session_runtime_mut(&session_id);
        crate::app::message::chat::processing_timing::pause_runtime_processing_timers(runtime)
    };

    let mut persisted_chat: Vec<models::ChatMessage> = if is_active_session {
        app.chat.clone()
    } else if let Some(cached) = app.session_chat_cache.get(&session_id).cloned() {
        cached.iter().cloned().collect::<Vec<models::ChatMessage>>()
    } else {
        load_session_or_default(app, session_id.clone()).messages
    };
    crate::app::message::chat::processing_timing::complete_tail_assistant_processing(
        &mut persisted_chat,
        completed_ms,
        elapsed.processing_ms,
        elapsed.llm_ms,
    );
    let terminal_tail =
        calibrate_stream_tail_at_terminal(app, &session_id, &mut persisted_chat, completed_ms);
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

    let mut local_session = load_session_or_default(app, session_id.clone());
    local_session.messages = persisted_chat.clone();
    local_session.message_ids = persisted_ids.clone();
    local_session.updated_ms = completed_ms;
    if let Some(turn_id) = assistant_message_id.as_deref() {
        for step in local_session.steps.iter_mut().filter(|step| {
            step.turn_id.is_none()
                && (request_started_ms == 0 || step.started_ms >= request_started_ms)
        }) {
            step.turn_id = Some(turn_id.to_string());
        }
        if is_active_session {
            app.replace_active_session_steps(local_session.steps.clone());
        }
    }
    let save_task = save_session_task(local_session, session_directory_for_save(app, &session_id));
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(persisted_chat),
        persisted_ids,
    );
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
        runtime.is_requesting = false;
        runtime.has_unseen_success = !is_active_session;
        runtime.active_agent_request = None;
    }
    app.sync_task_pet_from_runtime();
    app.show_send_mode_popover = false;

    if is_active_session {
        app.sync_active_session_from_chat();
    }

    let next = if handoff_task.is_some() { Task::none() } else { start_next(app, &session_id) };
    let refresh_message_ids = if is_active_session && !has_pending_queue && !resume_history_only {
        app.sessions
            .iter()
            .find(|entry| entry.id == session_id)
            .map(|entry| entry.directory.clone())
            .or_else(|| app.project_path.clone())
            .map(|project_path| {
                let session_id = session_id.clone();
                Task::perform(
                    async move {
                        let client = match crate::app::gateway_client() {
                            Ok(client) => client,
                            Err(err) => return Err(err),
                        };
                        let msgs = client
                            .session_messages::<Vec<message::WithParts>>(
                                &session_id,
                                Some(&project_path),
                            )
                            .await;
                        let info = client
                            .session_get::<session_info::Info>(&session_id, Some(&project_path))
                            .await;
                        match (msgs, info) {
                            (Ok(msgs), Ok(_)) => {
                                let mut usage = crate::app::models::TokenUsage::default();
                                for message in &msgs {
                                    if let message::Info::Assistant(assistant) = &message.info {
                                        usage.input_tokens += assistant.tokens.input;
                                        usage.output_tokens += assistant.tokens.output;
                                        usage.cached_tokens += assistant.tokens.cache.read
                                            + assistant.tokens.cache.write;
                                        usage.reasoning_tokens += assistant.tokens.reasoning;
                                    }
                                }
                                Ok((session_id, msgs, usage))
                            }
                            (Err(err), _) => Err(err),
                            (_, Err(err)) => Err(err),
                        }
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
            .or_else(|| app.project_path.clone());
        super::todos::load_input_panel_todos_task(session_id.clone(), directory)
    } else {
        Task::none()
    };
    let trajectory_raw_task = if is_active_session {
        super::super::trajectory::refresh_missing_raw_task(app, &session_id, None)
    } else {
        Task::none()
    };

    if is_active_session && app.chat_auto_scroll {
        scroll_chat_to_bottom_task(app)
    } else {
        Task::none()
    }
    .chain(session_todos_task)
    .chain(refresh_message_ids)
    .chain(save_task)
    .chain(trajectory_raw_task)
    .chain(next)
    .chain(handoff_task.unwrap_or_else(Task::none))
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
    let Some(active_request) = runtime.active_agent_request.as_ref().map(Arc::clone) else {
        return Task::none();
    };
    let insert_idx = leading_guide_count(&runtime.queue);
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
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );
    let mut session = load_session_or_default(app, session_id.clone());
    session.messages = app.chat.clone();
    session.message_ids = app.chat_message_ids.clone();
    session.updated_ms = completed_ms;
    let save_task = save_session_task(session, session_directory_for_save(app, &session_id));

    let resume_history = app.chat.clone();
    let continuation = QueueItem {
        created_ms: now_ms(),
        query: continuation_label_from_history(&resume_history),
        attachments: Vec::new(),
        root: active_request.root.clone(),
        model: active_request.model.clone(),
        reasoning_effort: active_request.reasoning_effort,
        acp_test: active_request.acp_test,
        acp_agent: active_request.acp_agent.clone(),
        acp_allowed_tools: active_request.acp_allowed_tools.clone(),
        agent: active_request.agent.clone(),
        allowed_tools: active_request.allowed_tools.clone(),
        require_agent_tool: active_request.require_agent_tool,
        acp_force_new_session: false,
        acp_history_mode: active_request.acp_history_mode,
        acp_recent_count: active_request.acp_recent_count,
        full_access_enabled: active_request.full_access_enabled,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: Some(resume_history),
        resume_history_only: true,
        workflow_mode_enabled: active_request.workflow_mode_enabled,
        goal_mode_enabled: active_request.goal_mode_enabled,
    };
    let next_item = {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.is_requesting = false;
        runtime.submit_anim = 0;
        runtime.has_unseen_success = false;
        runtime.active_agent_request = None;
        runtime.queue.insert(insert_idx, continuation);
        runtime.queue.remove(0)
    };

    app.sync_task_pet_from_runtime();
    app.show_send_mode_popover = false;
    app.sync_active_session_from_chat();
    info!(
        target: "vw_desktop",
        request_id = id,
        session_id = %session_id,
        step_index,
        queued_after_handoff = app.get_session_runtime(&session_id).queue.len(),
        "desktop yielded active request after tool round to prioritize guided queue item"
    );

    save_task.chain(chat_session::start(app, next_item, true))
}

fn is_payload_too_large_stream_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    err.contains("413")
        || lower.contains("payload too large")
        || lower.contains("length limit exceeded")
        || err.contains("请求内容过大")
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
            app.git_commit_in_progress = false;
        }
        app.quick_message_handoff = None;
    }
    let display_err = if is_payload_too_large_stream_error(&err) {
        "请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试"
            .to_string()
    } else {
        err
    };

    let Some(session_id) = app.find_session_by_request_id(id) else {
        // 请求态可能已因取消/切会话被清掉；仍用 toast 暴露错误，避免界面“假死”。
        crate::app::state::clear_pending_guide_handoff(id);
        if let Some(active_id) = app.active_session_id.clone() {
            let runtime = app.get_session_runtime_mut(&active_id);
            if runtime.is_requesting {
                runtime.is_requesting = false;
                runtime.has_unseen_success = false;
                runtime.active_agent_request = None;
                app.sync_task_pet_from_runtime();
            }
        }
        return app.show_error_toast(display_err);
    };
    let _ = flush_agent_stream_delta(app, session_id.clone());
    clear_agent_stream_session_save_flush(app, &session_id);
    crate::app::state::clear_pending_guide_handoff(id);
    let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
    let completed_ms = now_ms();

    let elapsed = {
        let runtime = app.get_session_runtime_mut(&session_id);
        let elapsed =
            crate::app::message::chat::processing_timing::pause_runtime_processing_timers(runtime);
        runtime.is_requesting = false;
        runtime.has_unseen_success = false;
        runtime.active_agent_request = None;
        elapsed
    };
    app.sync_task_pet_from_runtime();
    app.show_send_mode_popover = false;

    let toast_task = app.show_error_toast(display_err.clone());

    let mut persisted_chat = if is_active_session {
        app.chat.clone()
    } else if let Some(cached) = app.session_chat_cache.get(&session_id).cloned() {
        cached.iter().cloned().collect::<Vec<models::ChatMessage>>()
    } else {
        load_session_or_default(app, session_id.clone()).messages
    };
    let mut persisted_ids = if is_active_session {
        app.chat_message_ids.clone()
    } else {
        app.session_chat_message_id_cache
            .get(&session_id)
            .cloned()
            .unwrap_or_else(|| vec![None; persisted_chat.len()])
    };
    crate::app::message::chat::processing_timing::complete_tail_assistant_processing(
        &mut persisted_chat,
        completed_ms,
        elapsed.processing_ms,
        elapsed.llm_ms,
    );
    let terminal_tail =
        calibrate_stream_tail_at_terminal(app, &session_id, &mut persisted_chat, completed_ms);
    persisted_chat.push(models::ChatMessage {
        role: models::ChatRole::System,
        content: display_err,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    persisted_ids.push(None);
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
        app.sync_active_session_from_chat();
    }

    let mut session = load_session_or_default(app, session_id.clone());
    session.messages = persisted_chat;
    session.message_ids = persisted_ids;
    session.updated_ms = completed_ms;
    let save_task = save_session_task(session, session_directory_for_save(app, &session_id));

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
