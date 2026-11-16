//! 会话草稿、排队与一次请求的生命周期转换。
use super::request::{ChatSendBehavior, DispatchKind, QueueItem};
use super::session::compose_query_with_attachments;
use crate::session::agent::{SessionDraft, SessionRuntimeState};
use session_ui_types::ChatMessage;
use std::sync::Arc;
use tracing::info;

pub enum StartGate {
    PermissionQueued,
    HistoryQueued,
}
pub fn gate_start<R>(
    runtime: &mut SessionRuntimeState<R>,
    item: QueueItem,
    keep: bool,
    active: bool,
    ui_preparing: bool,
    base_ready: bool,
) -> Result<QueueItem, StartGate> {
    if runtime.full_access_saving || runtime.full_access_save_failed {
        runtime.request_lifecycle.queue.push(item);
        if !keep {
            runtime.input_editor = iced::widget::text_editor::Content::new();
        }
        return Err(StartGate::PermissionQueued);
    }
    if active && runtime.draft.is_none() && ui_preparing && !base_ready {
        runtime.request_lifecycle.queue.insert(0, item);
        return Err(StartGate::HistoryQueued);
    }
    Ok(item)
}

pub fn initialize_draft<R>(
    runtime: &mut SessionRuntimeState<R>,
    project_path: String,
    history_len: usize,
) {
    runtime.fork_execution_started = None;
    runtime.fork_request_submitted = false;
    runtime.fork_agent_selected = false;
    let directory = runtime
        .draft
        .as_ref()
        .filter(|draft| draft.project_path == project_path)
        .map_or_else(|| project_path.clone(), |draft| draft.directory.clone());
    runtime.draft = Some(SessionDraft { project_path, directory, history_len });
}

pub struct PreparedRequest {
    pub item: QueueItem,
    pub is_draft: bool,
    pub request_history: Vec<ChatMessage>,
    pub request_query: String,
    pub appended_start_idx: usize,
    pub appended_user_idx: Option<usize>,
}
pub fn prepare_request<R>(
    runtime: &mut SessionRuntimeState<R>,
    mut item: QueueItem,
    chat: &mut Vec<ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    request_started_ms: u64,
) -> PreparedRequest {
    // 主动停止后续接（包括队列中的“立刻引导”）保留已显示的历史。
    // 只有普通失败/取消重试才回到草稿的上一稳定边界。
    if item.send_behavior == ChatSendBehavior::StopAndSend {
        let history_len = chat.len();
        if let Some(draft) = runtime.draft.as_mut() {
            draft.history_len = history_len;
        }
    }
    let draft = runtime.draft.clone();
    let is_draft = draft.is_some();
    if let Some(draft) = draft {
        if runtime.full_access_overridden {
            item.full_access_enabled = runtime.full_access_enabled;
        }
        item.root = Some(draft.directory);
        // Failed first responses are still conversation history. Only an explicit
        // versioned history edit may remove them; retrying a draft must append.
    }
    let request_history = item.request_history_override.clone().unwrap_or_else(|| chat.clone());
    let request_query = if item.resume_history_only {
        String::new()
    } else {
        compose_query_with_attachments(&item.query, &item.attachments)
    };
    let message_model = super::session::normalize_message_model(item.model.as_deref());
    let (appended_start_idx, appended_user_idx) = super::session::append_request_messages(
        chat,
        message_ids,
        &request_query,
        message_model,
        request_started_ms,
    );
    PreparedRequest {
        item,
        is_draft,
        request_history,
        request_query,
        appended_start_idx,
        appended_user_idx,
    }
}
pub fn begin_request<R>(
    runtime: &mut SessionRuntimeState<R>,
    item: QueueItem,
    stream_id: u64,
    session_id: String,
    request_query: String,
    request_history: Vec<ChatMessage>,
    is_draft: bool,
    request_started_ms: u64,
    keep: bool,
) {
    runtime.request_lifecycle.begin_timing(request_started_ms, web_time::Instant::now());
    runtime.fork_request_submitted = runtime.fork_execution_started.is_some();
    runtime.has_unseen_success = false;
    info!(
        target: "vw_desktop",
        request_id = stream_id,
        session_id = %session_id,
        acp_test = item.acp_test,
        acp_agent = ?item.acp_agent,
        acp_allowed_tools = ?item.acp_allowed_tools,
        agent = ?item.agent,
        allowed_tools = ?item.allowed_tools,
        require_agent_tool = item.require_agent_tool,
        acp_force_new_session = item.acp_force_new_session,
        acp_history_mode = %item.acp_history_mode.as_str(),
        acp_recent_count = item.acp_recent_count,
        full_access_enabled = item.full_access_enabled,
        workflow_mode_enabled = item.workflow_mode_enabled,
        goal_mode_enabled = item.goal_mode_enabled,
        model = ?item.model,
        has_root = item.root.is_some(),
        query_len = request_query.len(),
        history_len = request_history.len(),
        resume_history_only = item.resume_history_only,
        keep_input = keep,
        "desktop agent request started"
    );
    let full_access_enabled = (is_draft && (runtime.full_access_initialized || item.acp_test))
        .then_some(item.full_access_enabled);
    runtime.request_lifecycle.active_agent_request = Some(Arc::new(item.into_request(
        stream_id,
        session_id.clone(),
        request_query,
        request_history,
        full_access_enabled,
    )));
    if !is_draft
        && runtime
            .request_lifecycle
            .queue
            .first()
            .is_some_and(|queued| queued.send_behavior == ChatSendBehavior::Guide)
    {
        crate::session::agent::mark_pending_guide_handoff(stream_id);
    } else {
        crate::session::agent::clear_pending_guide_handoff(stream_id);
    }
    if runtime
        .request_lifecycle
        .active_agent_request
        .as_ref()
        .is_some_and(|request| request.acp_test)
    {
        runtime.acp_rebuild_required = false;
    }
    runtime.last_effective_acp_agent = runtime
        .request_lifecycle
        .active_agent_request
        .as_ref()
        .and_then(|request| request.acp_agent.clone());
}

pub enum DispatchEffect {
    None,
    StopDraft,
    Start { item: QueueItem, replaced_request: Option<u64>, replace: bool },
}
pub fn dispatch<R>(
    runtime: &mut SessionRuntimeState<R>,
    item: QueueItem,
    active: bool,
) -> DispatchEffect {
    let kind = runtime
        .request_lifecycle
        .dispatch_kind(item.send_behavior, active && runtime.draft.is_some());
    match kind {
        DispatchKind::StopDraft => {
            runtime.request_lifecycle.queue.insert(0, item);
            runtime.input_editor = iced::widget::text_editor::Content::new();
            DispatchEffect::StopDraft
        }
        DispatchKind::Replace => {
            let replaced_request = active
                .then(|| {
                    runtime
                        .request_lifecycle
                        .active_agent_request
                        .as_ref()
                        .map(|request| request.id)
                })
                .flatten();
            if let Some(id) = replaced_request {
                crate::session::agent::clear_pending_guide_handoff(id);
            }
            if active {
                runtime.request_lifecycle.clear_request();
            }
            DispatchEffect::Start { item, replaced_request, replace: true }
        }
        DispatchKind::Guide => {
            if runtime.draft.is_none() {
                if let Some(request) = &runtime.request_lifecycle.active_agent_request {
                    crate::session::agent::mark_pending_guide_handoff(request.id);
                }
            }
            runtime.request_lifecycle.queue.insert(0, item);
            runtime.input_editor = iced::widget::text_editor::Content::new();
            DispatchEffect::None
        }
        DispatchKind::Queue => {
            runtime.request_lifecycle.queue.push(item);
            runtime.input_editor = iced::widget::text_editor::Content::new();
            DispatchEffect::None
        }
        DispatchKind::Start => {
            DispatchEffect::Start { item, replaced_request: None, replace: false }
        }
    }
}

/// 项目异步解析只允许恢复发起时仍活动的草稿。
pub fn pending_project_is_current(
    selected_project: Option<&str>,
    project: &str,
    current_stream: u64,
    stream: u64,
    active_session: Option<&str>,
) -> bool {
    selected_project == Some(project) && current_stream == stream && active_session.is_none()
}

pub fn inferred_project_id<'a>(
    current: Option<&str>,
    project: &str,
    project_sessions: impl Iterator<Item = &'a session_info::Info>,
    sessions: impl Iterator<Item = &'a session_info::Info>,
) -> Option<String> {
    current
        .map(str::to_owned)
        .or_else(|| project_sessions.take(1).next().map(|session| session.project_id.clone()))
        .or_else(|| {
            sessions
                .filter(|session| session.directory == project)
                .next()
                .map(|session| session.project_id.clone())
        })
}

pub fn restore_cached_chat(
    chat: &mut Vec<ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    cached: Option<&[ChatMessage]>,
    cached_ids: Option<Vec<Option<String>>>,
) {
    if let Some(cached) = cached {
        *chat = cached.to_vec();
        *message_ids = cached_ids
            .filter(|ids| ids.len() == chat.len())
            .unwrap_or_else(|| vec![None; chat.len()]);
    } else {
        chat.clear();
        message_ids.clear();
    }
}

pub fn session_snapshot(
    id: String,
    title: String,
    messages: Vec<ChatMessage>,
    message_ids: Vec<Option<String>>,
    now: u64,
) -> session_ui_types::ChatSession {
    session_ui_types::ChatSession {
        id,
        title,
        messages,
        message_ids,
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: now,
        updated_ms: now,
    }
}

pub fn retain_history(
    chat: &[ChatMessage],
    message_ids: &[Option<String>],
    index: usize,
) -> (Vec<ChatMessage>, Vec<Option<String>>) {
    let end = index.saturating_add(1);
    (
        chat.get(..end).unwrap_or_default().to_vec(),
        message_ids.get(..end).unwrap_or_default().to_vec(),
    )
}

pub async fn load_send_history(
    client: vw_gateway_client::GatewayClient,
    session_id: &str,
    directory: &str,
) -> Result<Box<session_ui_types::ChatSession>, String> {
    let loaded = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_ui_get_without_raw(session_id, Some(directory))
        .await?
        .ok_or_else(|| "会话历史不存在，未发送消息".to_string())?;
    if loaded.session.id != session_id {
        return Err("网关返回了其他会话的历史".to_string());
    }
    Ok(Box::new(loaded.session))
}

pub fn complete_cancel<R>(
    runtime: &mut SessionRuntimeState<R>,
    chat: &mut [ChatMessage],
    completed_ms: u64,
) {
    let lifecycle = &mut runtime.request_lifecycle;
    let elapsed = super::processing_timing::pause_runtime_processing_timers(
        &mut lifecycle.processing_timer,
        &mut lifecycle.llm_timer,
    );
    super::processing_timing::complete_tail_assistant_processing(
        chat,
        completed_ms,
        elapsed.processing_ms,
        elapsed.llm_ms,
    );
}

pub fn next_queued<R>(runtime: &mut SessionRuntimeState<R>) -> Option<QueueItem> {
    if runtime.request_lifecycle.queue.is_empty() {
        None
    } else {
        Some(runtime.request_lifecycle.queue.remove(0))
    }
}

pub enum ProjectResolutionEffect {
    Ignore,
    Resume { project_id: String, pending: super::session::PendingSessionSend },
    Failed { attachments: Vec<String>, error: String },
}
pub fn resolve_pending_send(
    current: bool,
    pending: super::session::PendingSessionSend,
    result: Result<String, String>,
) -> ProjectResolutionEffect {
    if !current {
        return ProjectResolutionEffect::Ignore;
    }
    match result {
        Ok(project_id) => ProjectResolutionEffect::Resume { project_id, pending },
        Err(error) => {
            ProjectResolutionEffect::Failed { attachments: pending.item.attachments, error }
        }
    }
}

pub fn send_gate<R>(
    runtime: Option<&SessionRuntimeState<R>>,
    active: bool,
    preparing: bool,
    ready: bool,
) -> Option<&'static str> {
    if runtime.is_some_and(|runtime| {
        runtime.acp_options_state.source_pending.is_some() || runtime.acp_options_state.pending
    }) {
        return Some("模型来源正在保存，请稍后发送");
    }
    if active && runtime.is_none_or(|runtime| runtime.draft.is_none()) && preparing && !ready {
        return Some("会话历史正在加载，请稍后发送");
    }
    None
}

pub enum InstallationEffect {
    Ignore,
    Error(String),
    Checked {
        statuses: std::collections::BTreeMap<String, bool>,
        warning: Option<&'static str>,
        send: bool,
    },
}
pub fn resolve_installation<R>(
    active_session: Option<&str>,
    checked_session: Option<&str>,
    runtime: &SessionRuntimeState<R>,
    agent: Option<&str>,
    backend: &str,
    query: Option<String>,
    result: Result<std::collections::BTreeMap<String, bool>, String>,
) -> InstallationEffect {
    if active_session != checked_session
        || runtime.agent.as_deref() != agent
        || runtime.acp_agent.as_deref() != Some(backend)
    {
        return InstallationEffect::Ignore;
    }
    let statuses = match result {
        Ok(statuses) => statuses,
        Err(error) => return InstallationEffect::Error(error),
    };
    let installed = statuses.get(backend).copied();
    let warning = match installed {
        Some(true) => None,
        Some(false) => Some("CLI 未安装，请通过输入区的安装 / 运行设置安装"),
        None => Some("网关未提供该智能体的安装状态，请检查运行设置"),
    };
    let send = warning.is_none() && query.is_some_and(|query| query == runtime.input_editor.text());
    InstallationEffect::Checked { statuses, warning, send }
}

pub enum HistoryLoadEffect {
    UseCached,
    Store(Box<session_ui_types::ChatSession>),
    Error(String),
}
pub fn resolve_history_load(
    session_id: &str,
    cached: bool,
    requesting: bool,
    result: Result<Box<session_ui_types::ChatSession>, String>,
) -> HistoryLoadEffect {
    let loaded = match result {
        Ok(loaded) if loaded.id == session_id => loaded,
        Ok(_) => return HistoryLoadEffect::Error("网关返回了其他会话的历史，未发送消息".into()),
        Err(error) => {
            return HistoryLoadEffect::Error(format!("加载会话历史失败，未发送消息：{error}"));
        }
    };
    if cached {
        HistoryLoadEffect::UseCached
    } else if requesting {
        HistoryLoadEffect::Error("会话正在回复且历史尚未就绪，请稍后发送".into())
    } else {
        HistoryLoadEffect::Store(loaded)
    }
}
