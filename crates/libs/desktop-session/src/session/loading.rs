//! 会话打开提交与历史摘要，首屏渲染由调用方提供。
use session_ui_types::{ChatMessage, ChatSession, ChatSessionMeta, ChatSessionStep, TokenUsage};
use crate::message::chat::trajectory::SessionStepRawKey;
use super::chat_timeline::{ChatEntryRevision, ChatTimelineEntry, TimelinePage};
use serde_json::Value;
use std::{collections::HashMap, hash::{Hash, Hasher}, sync::Arc};
/// 为 SharedChatMessages 领域数据提供更清晰的类型名称。
pub use super::cache::SharedChatMessages;

/// 冷会话打开在后台完成后的完整提交单元。
///
/// 消息与其首屏派生状态由同一个后台任务产生，主线程只负责校验目标会话并移动安装。
#[derive(Debug, Clone)]
pub struct PreparedSessionOpen<UiChunk> {
    pub session_id: String,
    pub agent_binding: Option<vw_api_chat_identity::SessionAgentIdentity>,
    pub full_access: Option<bool>,
    pub messages: Vec<ChatMessage>,
    pub message_ids: Vec<Option<String>>,
    pub usage: TokenUsage,
    pub steps: Vec<ChatSessionStep>,
    pub calls: Vec<Value>,
    pub step_llm_raw: HashMap<SessionStepRawKey, Value>,
    pub llm_raw_step_indices: Vec<SessionStepRawKey>,
    pub updated_ms: u64,
    pub content_signature: u64,
    pub session_model: Option<String>,
    pub base_ui_chunk: Option<UiChunk>,
    pub timeline_page: Option<TimelinePage>,
}

/// 计算会话可见内容与消息标识的稳定进程内签名。
pub fn session_content_signature(chat: &[ChatMessage], message_ids: &[Option<String>]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    chat.len().hash(&mut hasher);
    for (index, message) in chat.iter().enumerate() {
        index.hash(&mut hasher);
        message.hash(&mut hasher);
        message_ids.get(index).hash(&mut hasher);
    }
    hasher.finish()
}

/// 在后台将已转换的历史和首个 UI 分块封装为一次冷打开提交。
pub fn prepare_session_open<UiChunk>(
    session_id: String,
    messages: Vec<ChatMessage>,
    mut message_ids: Vec<Option<String>>,
    usage: TokenUsage,
    steps: Vec<ChatSessionStep>,
    calls: Vec<Value>,
    updated_ms: u64,
    session_model: Option<String>,
    prepare_base: impl FnOnce(&[ChatMessage]) -> UiChunk,
) -> PreparedSessionOpen<UiChunk> {
    if message_ids.len() != messages.len() {
        message_ids = vec![None; messages.len()];
    }
    let content_signature = session_content_signature(&messages, &message_ids);
    let base_ui_chunk = (!messages.is_empty()).then(|| prepare_base(&messages));

    PreparedSessionOpen {
        session_id,
        agent_binding: None,
        full_access: None,
        messages,
        message_ids,
        usage,
        steps,
        calls,
        step_llm_raw: HashMap::new(),
        llm_raw_step_indices: Vec::new(),
        updated_ms,
        content_signature,
        session_model,
        base_ui_chunk,
        timeline_page: None,
    }
}

/// 把 Gateway 的最近消息页转换为首屏提交，不读取步骤、调用或完整历史。
pub fn prepare_bounded_session_open<UiChunk>(
    summary: vw_api_session::GatewaySessionUiSummary,
    page: vw_api_session::GatewaySessionMessagePage<
        session_ui_types::SessionMessageRecord,
    >,
    prepare_base: impl FnOnce(&[ChatMessage]) -> UiChunk,
) -> PreparedSessionOpen<UiChunk> {
    let entries = page
        .items
        .into_iter()
        .map(|record| {
            ChatTimelineEntry::persisted(
                &summary.id,
                record.seq_no,
                record.message_id,
                ChatEntryRevision::INITIAL,
                record.message,
            )
        })
        .collect::<Vec<_>>();
    let messages = entries.iter().map(|entry| entry.message.clone()).collect::<Vec<_>>();
    let message_ids = entries
        .iter()
        .map(|entry| match &entry.key {
            super::chat_timeline::ChatEntryKey::PersistedId(id) => Some(id.clone()),
            _ => None,
        })
        .collect();
    let mut prepared = prepare_session_open(
        summary.id,
        messages,
        message_ids,
        TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        summary.updated_ms,
        None,
        prepare_base,
    );
    prepared.agent_binding = summary.agent_binding;
    prepared.full_access = summary.full_access;
    prepared.timeline_page = Some(TimelinePage {
        entries,
        before_cursor: page.next_cursor,
        after_cursor: page.previous_cursor,
        total_count: page.total_count,
    });
    prepared
}

pub fn prepare_timeline_page(
    session_id: &str,
    page: vw_api_session::GatewaySessionMessagePage<
        session_ui_types::SessionMessageRecord,
    >,
) -> TimelinePage {
    TimelinePage {
        entries: page
            .items
            .into_iter()
            .map(|record| {
                ChatTimelineEntry::persisted(
                    session_id,
                    record.seq_no,
                    record.message_id,
                    ChatEntryRevision::INITIAL,
                    record.message,
                )
            })
            .collect(),
        before_cursor: page.next_cursor,
        after_cursor: page.previous_cursor,
        total_count: page.total_count,
    }
}

/// 在后台将 UI Store 会话移动为统一的冷打开提交。
pub fn prepare_session_open_from_ui_store<UiChunk>(
    session: ChatSession,
    prepare_base: impl FnOnce(&[ChatMessage]) -> UiChunk,
) -> PreparedSessionOpen<UiChunk> {
    let ChatSession { id, messages, message_ids, calls, steps, updated_ms, .. } = session;
    let usage = session_usage_from_steps(&steps);
    let session_model = steps.iter().rev().find_map(|step| step.model.clone());
    prepare_session_open(
        id,
        messages,
        message_ids,
        usage,
        steps,
        calls,
        updated_ms,
        session_model,
        prepare_base,
    )
}

fn session_usage_from_steps(steps: &[ChatSessionStep]) -> TokenUsage {
    let mut usage = TokenUsage::default();
    for step in steps {
        usage.input_tokens += step.usage.input_tokens;
        usage.output_tokens += step.usage.output_tokens;
        usage.cached_tokens += step.usage.cached_tokens;
        usage.reasoning_tokens += step.usage.reasoning_tokens;
    }
    usage
}

/// 执行 shared_chat_messages 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn shared_chat_messages(chat: Vec<ChatMessage>) -> SharedChatMessages {
    Arc::from(chat)
}

/// 执行 build_session_previews 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn build_session_previews(
    previews: Vec<ChatSessionMeta>,
) -> std::collections::HashMap<String, String> {
    previews
        .into_iter()
        .filter_map(|meta| {
            let content = meta.last_content?;
            let line = session_text::initial_title_from_content(&content)?;
            Some((meta.id, desktop_workspace::views::project::utils::truncate_display_width(line, 120)))
        })
        .collect()
}

/// 执行 session_usage 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn session_usage(session: &ChatSession) -> TokenUsage {
    session_usage_from_steps(&session.steps)
}


pub async fn load_project_sessions(
    endpoint: gateway_endpoint::GatewayEndpoint,
    path: Option<String>, project_id: Option<String>,
) -> Result<(Vec<session_info::Info>, HashMap<String, String>, std::collections::HashSet<String>), anyhow::Error> {
    use desktop_workspace::views::project::utils::truncate_display_width;
    // Scope reads and writes belong to the endpoint selected when this load began.
    let client =
        vw_gateway_client::GatewayClient::new(endpoint).map_err(anyhow::Error::msg)?;
    let scope_id = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_scope_get(path.as_deref(), project_id.as_deref())
        .await
        .map_err(anyhow::Error::msg)?;
    gateway_session::GatewaySessionClient::new(client.transport().clone()).session_scope_put(scope_id.as_deref()).await.map_err(anyhow::Error::msg)?;

    let previews = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_ui_previews(scope_id.as_deref())
        .await
        .map_err(anyhow::Error::msg)?;
    let archived_session_ids = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_archived_get(scope_id.as_deref())
        .await
        .map_err(anyhow::Error::msg)?
        .into_iter()
        .collect();

    let preview_map = previews
        .into_iter()
        .filter_map(|meta| {
            let content = meta.last_content?;
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((
                    meta.id,
                    truncate_display_width(&trimmed.replace('\n', " "), 120),
                ))
            }
        })
        .collect::<std::collections::HashMap<_, _>>();

    let sessions = if let Some(p) = path {
        desktop_workspace::project_message::gateway::list_gateway_project_sessions(
            &client, &p,
        )
        .await
        .map_err(anyhow::Error::msg)?
    } else {
        gateway_session::GatewaySessionClient::new(client.transport().clone())
            .session_list::<Vec<session_info::Info>>(None)
            .await
            .map_err(anyhow::Error::msg)?
    };

    Ok((sessions, preview_map, archived_session_ids))
}

#[cfg(test)]
#[path="loading_tests.rs"]
mod loading_tests;

/// Replacement loading retains the archive route's authoritative metadata read.
pub enum ReplacementHistory { Archived, Deleted }
pub async fn load_replacement_history(
    client: &vw_gateway_client::GatewayClient, id: String, route: ReplacementHistory,
) -> Result<(String, Vec<message_types::WithParts>, TokenUsage), String> {
    let session = gateway_session::GatewaySessionClient::new(client.transport().clone());
    let messages = session.session_messages::<Vec<message_types::WithParts>>(&id, None).await;
    let info = match route {
        ReplacementHistory::Archived => session.session_get::<session_info::Info>(&id, None).await.map(|_| ()),
        ReplacementHistory::Deleted => Ok(()),
    };
    let messages = messages?;
    info?;
    let mut usage = TokenUsage::default();
    for message in &messages {
        if let message_types::Info::Assistant(info) = &message.info {
            usage.input_tokens += info.tokens.input;
            usage.output_tokens += info.tokens.output;
            usage.cached_tokens += info.tokens.cache.read + info.tokens.cache.write;
            usage.reasoning_tokens += info.tokens.reasoning;
        }
    }
    Ok((id, messages, usage))
}

pub enum SessionMutation {
    Rename { id: String, title: String },
    Archive { id: String },
    Delete { id: String },
}
/// Persist a lifecycle effect using the endpoint selected by the host.
pub async fn persist_session_mutation(client: &vw_gateway_client::GatewayClient,
    project_path: Option<&str>, mutation: SessionMutation) -> Result<(), String> {
    use vw_api_session::{GatewaySessionPatchBody, GatewaySessionPatchTime};
    let session = gateway_session::GatewaySessionClient::new(client.transport().clone());
    match mutation {
        SessionMutation::Rename { id, title } => session.session_update::<session_info::Info>(
            &id, project_path, &GatewaySessionPatchBody { title: Some(title), time: None }).await.map(|_| ()),
        SessionMutation::Archive { id } => session.session_update::<session_info::Info>(
            &id, project_path, &GatewaySessionPatchBody { title: None,
                time: Some(GatewaySessionPatchTime { archived: Some(runtime_time::now_ms()) }) }).await.map(|_| ()),
        SessionMutation::Delete { id } => session.session_delete(&id, project_path).await.map(|_| ()),
    }
}
