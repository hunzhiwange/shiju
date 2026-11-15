//! 会话请求内容与持久化消息边界。
use std::path::Path;
use desktop_chat_tools::state::ShijuSkillSelection;
use desktop_settings_agents::state::MAIN_AGENT_KEY;
use message_types as agent_message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineModeCommand {
    TaskMode,
    ChatMode,
}


pub fn parse_inline_mode_command(query: &str) -> Option<InlineModeCommand> {
    let command = query.split_whitespace().next()?.to_ascii_lowercase();
    match command.as_str() {
        "/task" => Some(InlineModeCommand::TaskMode),
        "/new" | "/clear" | "/session" => Some(InlineModeCommand::ChatMode),
        _ => None,
    }
}

pub fn normalize_delegate_agent(selected: Option<String>) -> Option<String> {
    selected.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty() && trimmed != MAIN_AGENT_KEY).then(|| trimmed.to_string())
    })
}

pub fn request_delegate_agent(selected: Option<String>) -> Option<String> {
    normalize_delegate_agent(selected).or_else(|| Some(MAIN_AGENT_KEY.to_string()))
}

pub fn compose_query_with_session_context(
    query: &str,
    selected_tools: &[String],
    selected_skills: &[String],
    selected_shiju_skills: &[ShijuSkillSelection],
) -> String {
    let trimmed = query.trim();
    if selected_tools.is_empty() && selected_skills.is_empty() && selected_shiju_skills.is_empty() {
        return trimmed.to_string();
    }
    let mut effective_skills = selected_skills.to_vec();
    if !selected_shiju_skills.is_empty()
        && !effective_skills.iter().any(|skill| skill.eq_ignore_ascii_case("shiju"))
    {
        effective_skills.push("shiju".to_string());
    }

    let mut lines = vec![
        "<session_control_selection>".to_string(),
        "用户在会话控制中选中了以下上下文。选中不代表已经执行；请在本轮需要时再主动使用。"
            .to_string(),
    ];

    if !selected_tools.is_empty() {
        lines.push(format!("工具：{}", selected_tools.join(", ")));
    }
    if !effective_skills.is_empty() {
        lines.push(format!("技能：{}", effective_skills.join(", ")));
    }
    if !selected_shiju_skills.is_empty() {
        let slugs =
            selected_shiju_skills.iter().map(|selection| selection.slug()).collect::<Vec<_>>();
        lines.push(format!("识矩功能：{}", slugs.join(", ")));
        lines.push(
            "除非用户明确询问识矩平台或识矩技能本身，否则默认将用户的问题理解为针对上述已选识矩功能；不要回答成对识矩本身的介绍或使用说明。"
                .to_string(),
        );
    }
    lines.push("</session_control_selection>".to_string());

    let context = lines.join("\n");
    if trimmed.is_empty() { context } else { format!("{trimmed}\n\n{context}") }
}

fn is_supported_image_attachment(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
            )
        })
        .unwrap_or(false)
}

pub fn compose_query_with_attachments(query: &str, attachments: &[String]) -> String {
    let trimmed = query.trim();
    let markers = attachments
        .iter()
        .map(|path| {
            if is_supported_image_attachment(path) {
                format!("[IMAGE:{path}]")
            } else {
                format!("[DOCUMENT:{path}]")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    match (trimmed.is_empty(), markers.is_empty()) {
        (true, true) => String::new(),
        (false, true) => trimmed.to_string(),
        (true, false) => markers,
        (false, false) => format!("{trimmed}\n\n{markers}"),
    }
}

pub fn message_id_for_index_in_ids(message_ids: &[Option<String>], msg_idx: usize) -> Option<String> {
    let previous_end = msg_idx.min(message_ids.len());
    message_ids
        .get(msg_idx)
        .and_then(Clone::clone)
        .or_else(|| message_ids.get(..previous_end)?.iter().rev().find_map(Clone::clone))
}

fn push_recovered_message_id(
    ids: &mut Vec<Option<String>>,
    has_content: &mut bool,
    message_id: &Option<String>,
) {
    if *has_content {
        ids.push(message_id.clone());
        *has_content = false;
    }
}

fn recovered_message_ids_from_gateway_messages(
    msgs: Vec<agent_message::WithParts>,
) -> Vec<Option<String>> {
    let mut ids = Vec::new();

    for message in msgs {
        let message_id = Some(message.info.id().to_string());
        let mut has_content = false;

        for part in message.parts {
            match part {
                agent_message::Part::Text(part) => {
                    has_content = has_content || !part.text.trim().is_empty();
                }
                agent_message::Part::Reasoning(part) => {
                    has_content = has_content || !part.text.trim().is_empty();
                }
                agent_message::Part::Tool(_) => {
                    push_recovered_message_id(&mut ids, &mut has_content, &message_id);
                    ids.push(message_id.clone());
                }
                agent_message::Part::File(_) => {
                    has_content = true;
                }
                _ => {}
            }
        }

        push_recovered_message_id(&mut ids, &mut has_content, &message_id);
    }

    ids
}

fn latest_persisted_user_message_id(messages: &[agent_message::WithParts]) -> Option<String> {
    messages
        .iter()
        .filter(|message| matches!(message.info, agent_message::Info::User(_)))
        .map(|message| message.info.id())
        .max()
        .map(str::to_string)
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;

/// 恢复网关历史对应的本地消息边界，不访问桌面状态。
pub async fn recover_reset_message_ids(
    endpoint: (String, u16),
    session_id: &str,
    directory: Option<&str>,
) -> Result<Vec<Option<String>>, String> {
    let client = vw_gateway_client::GatewayClient::new(
        gateway_endpoint::GatewayEndpoint::new(endpoint.0, endpoint.1),
    )?;
    let messages = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_messages::<Vec<agent_message::WithParts>>(session_id, directory)
        .await?;
    Ok(recovered_message_ids_from_gateway_messages(messages))
}

/// 使用宿主注入的认证客户端压缩会话，保持最新用户边界选择规则。
pub async fn summarize_session(
    client: vw_gateway_client::GatewayClient,
    session_id: &str,
    project_path: &str,
) -> Result<bool, String> {
    let session = gateway_session::GatewaySessionClient::new(client.transport().clone());
    let messages = session.session_messages::<Vec<agent_message::WithParts>>(
        session_id, Some(project_path),
    ).await?;
    let message_id = latest_persisted_user_message_id(&messages)
        .ok_or_else(|| "当前会话没有可用于压缩的用户消息".to_string())?;
    session.session_summarize(
        session_id, Some(project_path),
        &vw_api_session::GatewaySessionSummarizeBody { message_id },
    ).await
}

/// 项目身份解析期间保留的首次发送内容。
#[derive(Debug, Clone)]
pub struct PendingSessionSend {
    pub item: super::request::QueueItem,
    pub keep: bool,
}

pub fn git_commit_selection_context(
    files_selected: &[String],
    hunks_selected: &[(String, usize)],
    lines_selected: &[(String, usize)],
    old_lines_selected: &[(String, usize)],
) -> String {
    let files = if files_selected.is_empty() {
        "无".to_string()
    } else {
        files_selected.join(", ")
    };
    let hunks = hunks_selected
        .iter()
        .map(|(path, index)| format!("{path}#{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let new_lines = lines_selected
        .iter()
        .map(|(path, line)| format!("{path}:{line}"))
        .collect::<Vec<_>>()
        .join(", ");
    let old_lines = old_lines_selected
        .iter()
        .map(|(path, line)| format!("{path}:{line}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "整文件：{files}\nHunk：{}\n新版本行：{}\n旧版本行：{}",
        if hunks.is_empty() { "无" } else { &hunks },
        if new_lines.is_empty() { "无" } else { &new_lines },
        if old_lines.is_empty() { "无" } else { &old_lines },
    )
}


pub fn git_commit_generation_prompt(project_path: &str, target: &str) -> String {
    format!(
        "请为以下 Git 变更生成提交信息。\n仓库路径：{project_path}\n目标：{target}\n\n\
         返回严格 JSON，禁止 Markdown 代码块或额外说明：\
         {{\"title\":\"简洁的提交标题\",\"summary\":\"说明主要改动与目的的摘要\"}}。\
         title 和 summary 都不能为空；title 可使用合适的 Conventional Commit 前缀。"
    )
}

pub fn gateway_endpoint(host: &str, port: u16) -> (String, u16) {
    let host = host.trim();
    let host = if host.is_empty() { "127.0.0.1" } else { host };
    (host.to_string(), port)
}

/// 为已接受的请求追加用户消息与助手占位，返回宿主需刷新的索引。
pub fn append_request_messages(
    chat: &mut Vec<session_ui_types::ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
    request_query: &str,
    message_model: Option<String>,
    request_started_ms: u64,
) -> (usize, Option<usize>) {
    let appended_start_idx = chat.len();
    let appended_user_idx = if request_query.trim().is_empty() {
        None
    } else {
        chat.push(session_ui_types::ChatMessage {
            role: session_ui_types::ChatRole::User,
            content: request_query.to_string(),
            model: message_model.clone(),
            think_timing: Vec::new(),
            processing_timing: None,
        });
        message_ids.push(None);
        Some(appended_start_idx)
    };
    chat.push(session_ui_types::ChatMessage {
        role: session_ui_types::ChatRole::Assistant,
        content: String::new(),
        model: message_model,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    super::processing_timing::start_tail_assistant_processing(chat, request_started_ms);
    message_ids.push(None);
    (appended_start_idx, appended_user_idx)
}

pub fn session_info_is_compacting<'a>(
    mut sessions: impl Iterator<Item = &'a session_info::Info>,
    session_id: &str,
) -> bool {
    sessions.find(|info| info.id == session_id)
        .is_some_and(|info| info.time.compacting.is_some())
}

pub fn set_session_compacting<'a>(
    sessions: impl Iterator<Item = &'a mut session_info::Info>,
    session_id: &str,
    compacting: Option<u64>,
) {
    for info in sessions.filter(|info| info.id == session_id) {
        info.time.compacting = compacting;
    }
}

/// 显式历史重置的本地边界；持久化前再次比对网关历史。
pub struct HistoryReset {
    boundary: session_ui_types::ChatMessage,
    boundary_id: Option<String>,
    seq_no: u64,
    now: u64,
}

impl HistoryReset {
    pub fn new(
        boundary: Option<session_ui_types::ChatMessage>,
        boundary_id: Option<String>,
        seq_base: u64,
        history_len: usize,
        now: u64,
    ) -> Result<Self, String> {
        let boundary = boundary.ok_or_else(|| "无法重置到不存在的消息".to_string())?;
        let seq_no = seq_base + history_len.saturating_sub(1) as u64;
        Ok(Self { boundary, boundary_id, seq_no, now })
    }

    pub async fn persist(
        self,
        client: vw_gateway_client::GatewayClient,
        session_id: &str,
        root: Option<&str>,
    ) -> Result<(), String> {
        let Self { boundary, boundary_id, seq_no, now } = self;
            let stored = gateway_session::GatewaySessionClient::new(client.transport().clone())
                .session_ui_get_without_raw(session_id, root)
                .await?
                .ok_or_else(|| "会话历史不存在".to_string())?
                .session;
            let index = usize::try_from(seq_no).map_err(|error| error.to_string())?;
            let matches = stored.messages.get(index).is_some_and(|message| {
                message.role == boundary.role && message.content == boundary.content
            }) && boundary_id.as_ref().is_none_or(|id| {
                stored.message_ids.get(index).and_then(Option::as_ref) == Some(id)
            });
            if !matches {
                return Err("会话历史已变化，请重新加载后重置".to_string());
            }
            let delta = session_ui_types::SessionMessageDelta {
                session_id: session_id.to_string(),
                expected_updated_ms: stored.updated_ms,
                updated_ms: now.max(stored.updated_ms.saturating_add(1)),
                upserts: Vec::new(),
                truncate_after: Some(seq_no),
            };
            gateway_session::GatewaySessionClient::new(client.transport().clone()).session_message_delta(session_id, root, &delta).await.map(|_| ())
    }
}

pub fn normalize_model(model: &str) -> String {
    let model = model.trim();
    if model.is_empty() { "auto".to_string() } else { model.to_string() }
}

pub fn normalize_message_model(model: Option<&str>) -> Option<String> {
    model.map(str::trim).filter(|model| !model.is_empty()).map(str::to_string)
}

pub fn history_recent_count(raw: &str) -> usize {
    raw.trim().parse::<usize>().ok().unwrap_or(3).clamp(1, 20)
}

pub fn reconcile_reasoning_effort(
    reasoning_effort: &mut Option<vw_api_chat_identity::ReasoningEffortDto>,
    task_mode_reasoning_effort: &mut Option<vw_api_chat_identity::ReasoningEffortDto>,
    has_task_executor: bool,
    options: &[Option<vw_api_chat_identity::ReasoningEffortDto>],
) -> bool {
    let reset_effort = reasoning_effort.is_some() && !options.contains(reasoning_effort);
    if reset_effort { *reasoning_effort = None; }
    if !has_task_executor && !options.contains(task_mode_reasoning_effort) {
        *task_mode_reasoning_effort = None;
    }
    reset_effort
}
