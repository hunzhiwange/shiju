//! 会话列表摘要投影和显式停止请求。
use session_info::Info;

pub fn apply_summary<'a>(sessions: impl Iterator<Item = &'a mut Info>, info: &Info) {
    for session in sessions {
        if session.id == info.id {
            session.summary = info.summary.clone();
            session.time.updated = session.time.updated.max(info.time.updated);
        }
    }
}

pub fn apply_diff_summary<'a>(
    sessions: impl Iterator<Item = &'a mut Info>,
    session_id: &str,
    summary: &session_info::Summary,
    request_id: u64,
    active_request_id: Option<u64>,
) {
    if active_request_id.is_none_or(|id| id == request_id) {
        for session in sessions {
            if session.id == session_id {
                session.summary = Some(summary.clone());
            }
        }
    }
}

pub async fn stop_stream(
    client: &vw_gateway_client::GatewayClient,
    request_id: &str,
    workflow: bool,
) -> Result<bool, String> {
    if workflow {
        gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
            .task_plan_run_stream_stop(request_id)
            .await
    } else {
        gateway_session::GatewaySessionClient::new(client.transport().clone())
            .chat_stream_stop(request_id)
            .await
    }
}

pub fn directory_for_save(known: Option<String>, project: Option<&str>) -> Option<String> {
    known
        .filter(|directory| !directory.trim().is_empty())
        .or_else(|| project.filter(|directory| !directory.trim().is_empty()).map(str::to_string))
}

pub struct ActiveSnapshot<'a> {
    pub messages: &'a [session_ui_types::ChatMessage],
    pub message_ids: &'a [Option<String>],
    pub calls: &'a [serde_json::Value],
    pub steps: &'a [session_ui_types::ChatSessionStep],
}

pub fn load_snapshot(
    id: String,
    info: Option<&Info>,
    active: Option<ActiveSnapshot<'_>>,
    cached: Option<(Vec<session_ui_types::ChatMessage>, Option<Vec<Option<String>>>)>,
    now: u64,
) -> session_ui_types::ChatSession {
    let (title, created_ms, updated_ms) = info
        .map(|s| (s.title.clone(), s.time.created, s.time.updated))
        .unwrap_or_else(|| ("新会话".to_string(), now, 0));
    let (messages, message_ids, calls, steps) = if let Some(active) = active {
        (
            active.messages.to_vec(),
            active.message_ids.to_vec(),
            active.calls.to_vec(),
            active.steps.to_vec(),
        )
    } else if let Some((messages, ids)) = cached {
        let message_ids = ids
            .filter(|ids| ids.len() == messages.len())
            .unwrap_or_else(|| vec![None; messages.len()]);
        (messages, message_ids, Vec::new(), Vec::new())
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };
    session_ui_types::ChatSession {
        id,
        title,
        messages,
        message_ids,
        calls,
        steps,
        created_ms,
        updated_ms,
    }
}

/// 仅活动且没有权限保存屏障的会话可消费队首。
pub fn take_ready_request<R>(
    runtime: &mut crate::session::agent::SessionRuntimeState<R>,
    active: bool,
) -> Option<crate::message::chat::request::QueueItem> {
    if !active
        || runtime.request_lifecycle.is_requesting
        || runtime.full_access_saving
        || runtime.full_access_save_failed
    {
        return None;
    }
    if runtime.request_lifecycle.queue.is_empty() {
        None
    } else {
        Some(runtime.request_lifecycle.queue.remove(0))
    }
}

/// 取消结算额外清除提交动画，保留已提交的聊天历史。
pub fn cancel_runtime<R>(runtime: &mut crate::session::agent::SessionRuntimeState<R>) {
    super::stream_events::finish_runtime(runtime, false);
    runtime.request_lifecycle.submit_anim = 0;
}
