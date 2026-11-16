//! 流消息投影、工具块合并与终止标识处理。
use super::tail::StreamTailState;
use crate::message::chat::request::{ChatSendBehavior, QueueItem};
use crate::session::agent::SessionRuntimeState;
use serde::Deserialize;
use session_ui_types as models;

pub(crate) fn update_think_timing_from_delta(
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

pub fn apply_stream_done_message_ids(
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

pub fn message_ids_tail(message_ids: &[Option<String>]) -> Vec<Option<String>> {
    let tail_len = message_ids.len().min(4);
    message_ids[message_ids.len().saturating_sub(tail_len)..].to_vec()
}

#[derive(Deserialize)]
pub struct GeneratedGitCommitMessage {
    #[serde(alias = "subject")]
    pub title: String,
    #[serde(alias = "description", alias = "body")]
    pub summary: String,
}

pub fn parse_generated_git_commit_message(content: &str) -> Option<GeneratedGitCommitMessage> {
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

pub(crate) fn parse_tool_block_len(s: &str) -> Option<usize> {
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

pub(crate) fn tool_call_id_from_raw_block(raw: &str) -> Option<String> {
    let (_, rest) = raw.split_once('\n')?;
    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    value.get("tool_call_id").and_then(serde_json::Value::as_str).map(ToOwned::to_owned)
}

pub(crate) fn tool_block_start_allowed(content: &str, start: usize) -> bool {
    if start == 0 || content[..start].ends_with('\n') {
        return true;
    }
    let prefix = content[..start].trim_end_matches([' ', '\t']);
    prefix.ends_with(':') || prefix.ends_with('：')
}

pub(crate) fn upsert_tool_block_by_call_id(content: &mut String, raw_tool_block: &str) {
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

pub(crate) fn append_tool_block(content: &mut String, raw_tool_block: &str) {
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(raw_tool_block.trim_end());
    content.push('\n');
}

pub fn leading_guide_count(queue: &[QueueItem]) -> usize {
    queue.iter().take_while(|item| item.send_behavior == ChatSendBehavior::Guide).count()
}

pub(crate) fn continuation_label_from_history(history: &[models::ChatMessage]) -> String {
    history
        .iter()
        .rev()
        .find(|message| matches!(message.role, models::ChatRole::User))
        .map(|message| message.content.clone())
        .unwrap_or_default()
}

/// 保证缓冲文本后的工具块从独立行开始；队列调度仍由调用方负责。
pub fn prepare_buffered_tool_line(pending: &mut String) {
    if !pending.is_empty() && !pending.ends_with('\n') {
        pending.push('\n');
    }
}

pub fn append_stream_delta_to_tail(
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

pub fn upsert_workflow_tool_to_tail(
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

pub(crate) fn instruction_read_tool_block(path: &str) -> String {
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

pub fn is_payload_too_large_stream_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    err.contains("413")
        || lower.contains("payload too large")
        || lower.contains("length limit exceeded")
        || err.contains("请求内容过大")
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalTailCalibration {
    pub message_idx: usize,
    pub think_count: usize,
    pub thinking_open: bool,
    pub estimated_height: f32,
}

pub fn calibrate_terminal_tail(
    state: &mut StreamTailState,
    message: &mut models::ChatMessage,
    message_idx: usize,
    think_count: usize,
    thinking_open: bool,
    estimated_height: f32,
    now: u64,
) -> Option<TerminalTailCalibration> {
    message.think_timing.truncate(think_count);
    state.calibrate(
        message_idx,
        message.content.len(),
        think_count,
        thinking_open,
        estimated_height,
    );
    state.advance_ui_generation();
    update_think_timing_from_delta(message, state, now);
    state.clear_tail();
    Some(TerminalTailCalibration { message_idx, think_count, thinking_open, estimated_height })
}
pub fn upsert_compaction_status(
    chat: &mut Vec<models::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    completed: bool,
    pending_index: Option<usize>,
    started_text: &str,
    completed_text: &str,
) -> usize {
    let content = if completed { completed_text } else { started_text };
    if completed
        && let Some(idx) = pending_index
        && let Some(message) = chat.get_mut(idx)
        && message.role == models::ChatRole::System
        && message.content == started_text
    {
        message.content = content.to_string();
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

pub fn continuation_item(
    active_request: &crate::message::chat::request::AgentRequest,
    resume_history: Vec<models::ChatMessage>,
    now: u64,
) -> QueueItem {
    QueueItem {
        created_ms: now,
        query: continuation_label_from_history(&resume_history),
        attachments: Vec::new(),
        root: active_request.root.clone(),
        model: active_request.model.clone(),
        reasoning_effort: active_request.reasoning_effort,
        acp_test: active_request.acp_test,
        acp_model_source: active_request.acp_model_source,
        acp_agent: active_request.acp_agent.clone(),
        acp_allowed_tools: active_request.acp_allowed_tools.clone(),
        agent: active_request.agent.clone(),
        allowed_tools: active_request.allowed_tools.clone(),
        require_agent_tool: active_request.require_agent_tool,
        acp_force_new_session: false,
        acp_history_mode: active_request.acp_history_mode,
        acp_recent_count: active_request.acp_recent_count,
        full_access_enabled: active_request.full_access_enabled.unwrap_or(false),
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: Some(resume_history),
        resume_history_only: true,
        workflow_mode_enabled: active_request.workflow_mode_enabled,
        goal_mode_enabled: active_request.goal_mode_enabled,
    }
}

pub fn refreshed_usage(messages: &[message_types::WithParts]) -> models::TokenUsage {
    let mut usage = models::TokenUsage::default();
    for message in messages {
        if let message_types::Info::Assistant(assistant) = &message.info {
            usage.input_tokens += assistant.tokens.input;
            usage.output_tokens += assistant.tokens.output;
            usage.cached_tokens += assistant.tokens.cache.read + assistant.tokens.cache.write;
            usage.reasoning_tokens += assistant.tokens.reasoning;
        }
    }
    usage
}

pub fn stream_delta_flush_interval(
    active_session_id: Option<&str>,
    session_id: &str,
) -> std::time::Duration {
    std::time::Duration::from_millis(if active_session_id == Some(session_id) { 16 } else { 100 })
}

pub enum DeltaEffect {
    Immediate(String),
    Schedule,
    Buffered,
}
pub fn queue_delta(
    tail: &StreamTailState,
    pending: &mut String,
    scheduled: &mut bool,
    is_active: bool,
    delta: String,
) -> DeltaEffect {
    if is_active && tail.message_idx().is_none() {
        *scheduled = false;
        let mut immediate = std::mem::take(pending);
        immediate.push_str(&delta);
        return DeltaEffect::Immediate(immediate);
    }
    pending.push_str(&delta);
    if *scheduled {
        DeltaEffect::Buffered
    } else {
        *scheduled = true;
        DeltaEffect::Schedule
    }
}
pub fn settle_compaction(
    chat: &mut [models::ChatMessage],
    pending_index: &mut Option<usize>,
    started_text: &str,
    content: &str,
) -> Option<usize> {
    let index = pending_index.take()?;
    let message = chat.get_mut(index)?;
    if message.role != models::ChatRole::System || message.content != started_text {
        return None;
    }
    message.content = content.into();
    Some(index)
}
pub fn append_system_notice(
    chat: &mut Vec<models::ChatMessage>,
    ids: &mut Vec<Option<String>>,
    content: String,
) -> usize {
    ids.resize(chat.len(), None);
    let index = chat.len();
    chat.push(models::ChatMessage {
        role: models::ChatRole::System,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    ids.push(None);
    index
}
pub fn instruction_read_tool_blocks(paths: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|path| !path.trim().is_empty())
        .filter(|path| seen.insert(path.clone()))
        .map(|path| instruction_read_tool_block(&path))
        .collect()
}

/// Buffer ownership and flush transitions belong to the session, independent of UI scheduling.
pub fn queue_runtime_delta<R>(
    runtime: &mut SessionRuntimeState<R>,
    active: bool,
    delta: String,
) -> DeltaEffect {
    queue_delta(
        &runtime.stream_tail_state,
        &mut runtime.pending_stream_delta,
        &mut runtime.stream_delta_flush_scheduled,
        active,
        delta,
    )
}
pub fn flush_runtime_delta<R>(runtime: &mut SessionRuntimeState<R>) -> String {
    runtime.stream_delta_flush_scheduled = false;
    std::mem::take(&mut runtime.pending_stream_delta)
}
pub fn apply_tool_blocks(
    chat: &mut Vec<models::ChatMessage>,
    ids: &mut Vec<Option<String>>,
    tail: &mut StreamTailState,
    blocks: Vec<String>,
) -> usize {
    let mut changed = chat.len().saturating_sub(1);
    for block in blocks {
        let target = chat
            .last()
            .filter(|message| message.role == models::ChatRole::Assistant)
            .map_or(chat.len(), |_| chat.len().saturating_sub(1));
        tail.begin_message(target);
        tail.apply_delta(&block);
        changed = upsert_workflow_tool_to_tail(chat, ids, block);
    }
    changed
}
pub fn apply_runtime_compaction<R>(
    runtime: &mut SessionRuntimeState<R>,
    chat: &mut Vec<models::ChatMessage>,
    ids: &mut Vec<Option<String>>,
    completed: bool,
    started: &str,
    finished: &str,
) -> usize {
    let pending = runtime.pending_compaction_status_index.take();
    let index = upsert_compaction_status(chat, ids, completed, pending, started, finished);
    runtime.pending_compaction_status_index = (!completed).then_some(index);
    index
}
pub fn finish_runtime<R>(runtime: &mut SessionRuntimeState<R>, unseen_success: bool) {
    runtime.request_lifecycle.is_requesting = false;
    runtime.has_unseen_success = unseen_success;
    runtime.request_lifecycle.active_agent_request = None;
}
/// A stale error may clear an orphaned busy flag, but cannot finish a newer request.
pub fn settle_unmatched_error<R>(runtime: &mut SessionRuntimeState<R>) -> bool {
    if runtime.request_lifecycle.active_agent_request.is_some() {
        return false;
    }
    if runtime.request_lifecycle.is_requesting {
        runtime.request_lifecycle.is_requesting = false;
        runtime.has_unseen_success = false;
    }
    true
}
pub fn yield_tool_round<R>(
    runtime: &mut SessionRuntimeState<R>,
    continuation: QueueItem,
    insert_index: usize,
) -> QueueItem {
    finish_runtime(runtime, false);
    runtime.request_lifecycle.submit_anim = 0;
    runtime.request_lifecycle.queue.insert(insert_index, continuation);
    runtime.request_lifecycle.queue.remove(0)
}
pub fn start_plan_runtime<R>(runtime: &mut SessionRuntimeState<R>) {
    runtime.is_plan_run_session = true;
    runtime.request_lifecycle.is_requesting = true;
}
pub fn plan_initial_chat(query: String) -> Vec<models::ChatMessage> {
    if query.trim().is_empty() {
        Vec::new()
    } else {
        vec![models::ChatMessage {
            role: models::ChatRole::User,
            content: query,
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }]
    }
}
pub fn model_recovery_matches<R>(
    runtime: &SessionRuntimeState<R>,
    id: u64,
    session_id: &str,
    event: &gateway_chat_stream::GatewayModelRecoveryEvent,
) -> bool {
    event.session_id == session_id
        && !runtime
            .model_recovery
            .as_ref()
            .is_some_and(|(request, previous)| *request == id && previous == event)
}
pub fn complete_draft_boundary<R>(
    runtime: &mut SessionRuntimeState<R>,
    terminal_index: Option<usize>,
) {
    if let (Some(last), Some(draft)) = (terminal_index, runtime.draft.as_mut()) {
        draft.history_len = last + 1;
    }
}
pub fn bind_terminal_turn(
    steps: &mut [models::ChatSessionStep],
    turn_id: Option<&str>,
    request_started_ms: u64,
) {
    if let Some(turn_id) = turn_id {
        for step in steps.iter_mut().filter(|step| {
            step.turn_id.is_none()
                && (request_started_ms == 0 || step.started_ms >= request_started_ms)
        }) {
            step.turn_id = Some(turn_id.to_string());
        }
    }
}

/// Terminal settlement inputs are local to one session; host effects run only after settlement.
pub struct TerminalRequest {
    pub has_pending_queue: bool,
    pub request_started_ms: u64,
    pub resume_history_only: bool,
    pub generation: u64,
    pub elapsed: crate::message::chat::processing_timing::ProcessingElapsed,
}
pub fn begin_terminal<R>(runtime: &mut SessionRuntimeState<R>) -> TerminalRequest {
    runtime.stream_save.flush_scheduled = false;
    let lifecycle = &mut runtime.request_lifecycle;
    TerminalRequest {
        has_pending_queue: !lifecycle.queue.is_empty(),
        request_started_ms: lifecycle.request_started_ms,
        resume_history_only: lifecycle
            .active_agent_request
            .as_ref()
            .is_some_and(|request| request.resume_history_only),
        generation: lifecycle.active_agent_request.as_ref().map_or(0, |request| request.id),
        elapsed: crate::message::chat::processing_timing::pause_runtime_processing_timers(
            &mut lifecycle.processing_timer,
            &mut lifecycle.llm_timer,
        ),
    }
}
pub fn complete_terminal_messages(
    chat: &mut [models::ChatMessage],
    terminal: &TerminalRequest,
    completed_ms: u64,
) {
    crate::message::chat::processing_timing::complete_tail_assistant_processing(
        chat,
        completed_ms,
        terminal.elapsed.processing_ms,
        terminal.elapsed.llm_ms,
    );
}
pub struct AppliedDelta {
    pub changed_index: usize,
    pub previous_content_len: Option<usize>,
    pub boundary_changed: bool,
}
pub fn apply_delta(
    chat: &mut Vec<models::ChatMessage>,
    ids: &mut Vec<Option<String>>,
    tail: &mut StreamTailState,
    delta: String,
    now: u64,
) -> AppliedDelta {
    let previous_content_len = chat
        .last()
        .filter(|message| message.role == models::ChatRole::Assistant)
        .map(|message| message.content.len());
    let previous_boundary = tail.text_block_boundary();
    let changed_index = append_stream_delta_to_tail(chat, ids, delta, now, tail);
    AppliedDelta {
        changed_index,
        previous_content_len,
        boundary_changed: tail.text_block_boundary() != previous_boundary,
    }
}
pub fn tail_target(chat: &[models::ChatMessage]) -> usize {
    chat.last()
        .filter(|message| message.role == models::ChatRole::Assistant)
        .map_or(chat.len(), |_| chat.len().saturating_sub(1))
}

pub enum CommitHandoff {
    Unrelated,
    Warning(&'static str),
    Commit {
        title: String,
        summary: String,
        action: crate::message::chat::git_handoff::QuickMessageGitCommitAction,
    },
}
pub fn complete_commit_handoff(
    state: Option<&crate::message::chat::git_handoff::QuickMessageHandoffState>,
    request_id: u64,
    session_id: &str,
    visible_prompt: Option<&str>,
    current_project: Option<&str>,
) -> CommitHandoff {
    use crate::message::chat::git_handoff::QuickMessageHandoffState;
    let Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
        request_id: pending,
        source_session_id,
        project_path,
        action,
    }) = state
    else {
        return CommitHandoff::Unrelated;
    };
    if *pending != request_id || source_session_id != session_id {
        return CommitHandoff::Unrelated;
    }
    let Some(generated) = visible_prompt.and_then(parse_generated_git_commit_message) else {
        return CommitHandoff::Warning("AI 未生成有效的提交标题和摘要，未执行提交");
    };
    if current_project != Some(project_path.as_str()) {
        return CommitHandoff::Warning("Git 工作树已切换，未执行提交");
    }
    CommitHandoff::Commit {
        title: generated.title.trim().into(),
        summary: generated.summary.trim().into(),
        action: action.clone(),
    }
}
pub fn display_stream_error(error: String) -> String {
    if is_payload_too_large_stream_error(&error) {
        "请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试"
            .into()
    } else {
        error
    }
}
pub struct CompletionEffects {
    pub notify: bool,
    pub start_next: bool,
    pub refresh_message_ids: bool,
}
pub fn completion_effects(
    active: bool,
    pending_queue: bool,
    resume_only: bool,
    draft: bool,
    handoff: bool,
    notifications: bool,
) -> CompletionEffects {
    CompletionEffects {
        notify: notifications && !resume_only && !handoff,
        start_next: !handoff,
        refresh_message_ids: active && !pending_queue && !resume_only && !draft,
    }
}

pub async fn reload_terminal_messages(
    client: &vw_gateway_client::GatewayClient,
    session_id: String,
    directory: &str,
) -> Result<(String, Vec<message_types::WithParts>, models::TokenUsage), String> {
    let gateway = gateway_session::GatewaySessionClient::new(client.transport().clone());
    let messages = gateway
        .session_messages::<Vec<message_types::WithParts>>(&session_id, Some(directory))
        .await;
    let info = gateway.session_get::<session_info::Info>(&session_id, Some(directory)).await;
    match (messages, info) {
        (Ok(messages), Ok(_)) => {
            let usage = refreshed_usage(&messages);
            Ok((session_id, messages, usage))
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}
