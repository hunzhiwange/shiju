//! 处理聊天会话相关应用消息。
//! 本模块协调用户输入、会话缓存和网关持久化。

use super::ChatMessage;
use super::ForkSessionLoaded;
use super::ForkSessionTarget;

use crate::app::message::TaskBoardMessage;
use crate::app::message::chat::stream;
use crate::app::state::{
    ChatSendBehavior, MAIN_AGENT_KEY, QuickMessageGitCommitAction, QuickMessageHandoffState,
    SessionDraft, SessionToolSelectorTab,
};
use crate::app::{AgentRequest, App, Message, QueueItem, message, models};
use iced::Task;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};
use vw_gateway_client::vw_api_types::acp_model_source::AcpModelSource;
use vw_shared::id;
use vw_shared::message::types as agent_message;
use vw_shared::session::info as session;

/// 项目身份解析期间保留的首次发送内容。
#[derive(Debug, Clone)]
pub struct PendingSessionSend {
    pub(crate) item: QueueItem,
    pub(crate) keep: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineModeCommand {
    TaskMode,
    ChatMode,
}

fn clear_quick_message_handoff_for_request(app: &mut App, request_id: u64) {
    let generating_git_commit = matches!(
        app.quick_message_handoff,
        Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
            request_id: pending_id,
            ..
        }) if pending_id == request_id
    );
    if matches!(
        app.quick_message_handoff,
        Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
            request_id: pending_id,
            ..
        }) if pending_id == request_id
    ) {
        app.quick_message_handoff = None;
        if generating_git_commit {
            app.git_commit_in_progress = false;
        }
    }
}

fn parse_inline_mode_command(query: &str) -> Option<InlineModeCommand> {
    let command = query.split_whitespace().next()?.to_ascii_lowercase();
    match command.as_str() {
        "/task" => Some(InlineModeCommand::TaskMode),
        "/new" | "/clear" | "/session" => Some(InlineModeCommand::ChatMode),
        _ => None,
    }
}

fn apply_inline_mode_command(app: &mut App, command: InlineModeCommand) -> Task<Message> {
    let runtime_editor = {
        let runtime = app.current_session_runtime_mut();
        runtime.task_mode_enabled = command == InlineModeCommand::TaskMode;
        runtime.input_editor = iced::widget::text_editor::Content::new();
        runtime.input_editor.clone()
    };
    if app.active_session_id.is_none() {
        app.input_editor = runtime_editor;
    }
    Task::none()
}

fn gateway_endpoint(app: &App) -> (String, u16) {
    let host = app.gateway_settings.host_input.trim();
    let host = if host.is_empty() { "127.0.0.1" } else { host };
    (host.to_string(), app.gateway_settings.port)
}

fn current_session_directory(app: &App) -> Option<String> {
    app.active_session_id.as_deref().and_then(|id| app.known_session_directory(id))
}

fn normalize_delegate_agent(selected: Option<String>) -> Option<String> {
    selected.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty() && trimmed != MAIN_AGENT_KEY).then(|| trimmed.to_string())
    })
}

fn request_delegate_agent(selected: Option<String>) -> Option<String> {
    normalize_delegate_agent(selected).or_else(|| Some(MAIN_AGENT_KEY.to_string()))
}

fn base_tools_for_selection(app: &mut App) -> Vec<String> {
    if !app.current_session_has_started() {
        app.current_session_runtime_mut().agent = None;
    }
    app.session_tool_inventory(&app.current_session_runtime()).base_tools
}

fn compose_query_with_session_context(
    query: &str,
    selected_tools: &[String],
    selected_skills: &[String],
    selected_shiju_skills: &[crate::app::state::ShijuSkillSelection],
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

fn dispatch_queue_item(app: &mut App, item: QueueItem) -> Task<Message> {
    let is_requesting = app.current_session_is_requesting();
    match item.send_behavior {
        ChatSendBehavior::StopAndSend => {
            if is_requesting
                && let Some(session_id) = app.active_session_id.clone()
                && app.current_session_runtime().draft.is_some()
            {
                let runtime = app.get_session_runtime_mut(&session_id);
                runtime.queue.insert(0, item);
                runtime.input_editor = iced::widget::text_editor::Content::new();
                return stream::stop_agent_stream(app, &session_id);
            }
            if let Some(session_id) = app.active_session_id.clone() {
                let request_id = app
                    .get_session_runtime(&session_id)
                    .active_agent_request
                    .as_ref()
                    .map(|request| request.id);
                if let Some(request_id) = request_id {
                    crate::app::state::clear_pending_guide_handoff(request_id);
                    clear_quick_message_handoff_for_request(app, request_id);
                }
                let runtime = app.get_session_runtime_mut(&session_id);
                runtime.is_requesting = false;
                runtime.submit_anim = 0;
                runtime.active_agent_request = None;
            }
            app.sync_task_pet_from_runtime();
            app.show_send_mode_popover = false;
            start(app, item, false)
        }
        ChatSendBehavior::Guide if is_requesting => {
            debug!(
                target: "vw_desktop",
                active_session_id = ?app.active_session_id,
                queue_len = app.current_session_runtime().queue.len() + 1,
                "desktop chat request guided ahead of queued requests"
            );
            if let Some(request_id) = app
                .current_session_runtime()
                .active_agent_request
                .as_ref()
                .map(|request| request.id)
                && app.current_session_runtime_ref().is_none_or(|runtime| runtime.draft.is_none())
            {
                crate::app::state::mark_pending_guide_handoff(request_id);
            }
            let runtime = app.current_session_runtime_mut();
            runtime.queue.insert(0, item);
            runtime.input_editor = iced::widget::text_editor::Content::new();
            Task::none()
        }
        ChatSendBehavior::Queue if is_requesting => {
            debug!(
                target: "vw_desktop",
                active_session_id = ?app.active_session_id,
                queue_len = app.current_session_runtime().queue.len() + 1,
                "desktop chat request queued while another request is active"
            );
            let runtime = app.current_session_runtime_mut();
            runtime.queue.push(item);
            runtime.input_editor = iced::widget::text_editor::Content::new();
            Task::none()
        }
        ChatSendBehavior::Guide | ChatSendBehavior::Queue => start(app, item, false),
    }
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

fn compose_query_with_attachments(query: &str, attachments: &[String]) -> String {
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

fn persist_acp_history_preferences(
    app: &mut App,
    mode: crate::app::state::AcpHistoryReplayMode,
    recent_count: usize,
) -> Task<Message> {
    app.acp_history_mode = mode;
    app.acp_recent_count = recent_count.clamp(1, 20);

    app.save_desktop_preferences(serde_json::Map::from_iter([
        ("acp_history_strategy".to_string(), serde_json::json!(mode.as_str())),
        ("acp_history_recent_count".to_string(), serde_json::json!(app.acp_recent_count)),
    ]))
}

fn persist_session_reasoning_effort(app: &mut App) -> Task<Message> {
    let Some(session_id) = app.active_session_id.clone() else {
        return Task::none();
    };
    let effort = app.current_session_runtime().reasoning_effort;
    app.session_reasoning_efforts.insert(session_id.clone(), effort);
    app.save_desktop_preference(
        "session_reasoning_efforts",
        serde_json::json!({session_id: effort}),
    )
}

fn message_id_for_index(app: &App, msg_idx: usize) -> Option<String> {
    message_id_for_index_in_ids(&app.chat_message_ids, msg_idx)
}

fn message_id_for_index_in_ids(message_ids: &[Option<String>], msg_idx: usize) -> Option<String> {
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

fn recover_reset_message_ids_task(
    endpoint: (String, u16),
    session_id: String,
    directory: Option<String>,
    msg_idx: usize,
) -> Task<Message> {
    Task::perform(
        async move {
            let result = async {
                let client = vw_gateway_client::GatewayClient::new(
                    vw_gateway_client::GatewayEndpoint::new(endpoint.0, endpoint.1),
                )?;
                let msgs = client
                    .session_messages::<Vec<agent_message::WithParts>>(
                        &session_id,
                        directory.as_deref(),
                    )
                    .await?;
                Ok(recovered_message_ids_from_gateway_messages(msgs))
            }
            .await;
            ChatMessage::ResetMessageIdsRecovered { session_id, msg_idx, result }
        },
        Message::Chat,
    )
}

fn reset_runtime_history_snapshots(
    app: &mut App,
    session_id: &str,
    retained_chat: &[models::ChatMessage],
) {
    let runtime = app.get_session_runtime_mut(session_id);
    if let Some(request) = runtime.active_agent_request.as_mut() {
        Arc::make_mut(request).history = retained_chat.to_vec();
    }
    for item in &mut runtime.queue {
        if item.resume_history_only || item.request_history_override.is_some() {
            item.request_history_override = Some(retained_chat.to_vec());
        }
    }
}

fn reset_session_locally(app: &mut App, session_id: String, msg_idx: usize) -> Task<Message> {
    let root = current_session_directory(app).or_else(|| app.project_path.clone());
    let retain_end = msg_idx.saturating_add(1);
    let retained_chat = app.chat.get(..retain_end).map(|slice| slice.to_vec()).unwrap_or_default();
    let retained_message_ids =
        app.chat_message_ids.get(..retain_end).map(|slice| slice.to_vec()).unwrap_or_default();
    let local_title = app
        .sessions
        .iter()
        .find(|session| session.id == session_id)
        .map(|session| session.title.clone())
        .unwrap_or_else(|| "重置会话".to_string());

    app.chat_reset_menu_idx = None;
    app.chat_fork_anchor = None;
    app.mark_session_acp_rebuild_required(&session_id);
    app.active_session_id = Some(session_id.clone());
    app.chat = retained_chat;
    app.chat_message_ids = retained_message_ids;
    app.mark_session_history_reset_boundary(session_id.clone(), app.chat_message_ids.clone());
    let retained_history = app.chat.clone();
    reset_runtime_history_snapshots(app, &session_id, &retained_history);
    app.invalidate_chat_ui_state();
    app.sync_active_session_preferences();
    app.rebuild_active_session_message_meta();
    app.store_session_chat_snapshot(
        session_id.clone(),
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );
    app.sync_active_session_from_chat();
    let history_len = app.chat.len();
    if let Some(draft) = app.get_session_runtime_mut(&session_id).draft.as_mut() {
        draft.history_len = draft.history_len.min(history_len);
        return Task::none();
    }

    let now =
        crate::app::time::now_ms().max(app.active_session_view_state.updated_ms.saturating_add(1));
    app.active_session_view_state.updated_ms = now;
    let local = models::ChatSession {
        id: session_id,
        title: local_title,
        messages: app.chat.clone(),
        message_ids: app.chat_message_ids.clone(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: now,
        updated_ms: now,
    };
    save_session_task(local, root)
}

fn save_session_task(session: models::ChatSession, directory: Option<String>) -> Task<Message> {
    // Stream updates and lifecycle events can enqueue snapshots at the same time.
    // Route this path through the same per-session coordinator so an older
    // snapshot cannot finish after a newer one and overwrite the history.
    stream::save_session_task(session, directory)
}

fn restore_cached_chat_for_session(app: &mut App, session_id: &str) {
    if let Some(cached) = app.cached_chat_messages(session_id) {
        app.chat = cached.iter().cloned().collect();
        app.chat_message_ids = app
            .cached_chat_message_ids(session_id)
            .filter(|ids| ids.len() == app.chat.len())
            .unwrap_or_else(|| vec![None; app.chat.len()]);
    } else {
        app.chat.clear();
        app.chat_message_ids.clear();
    }
}

fn send_to_existing_session(app: &mut App, session_id: String, input: String) -> Task<Message> {
    let input = input.trim().to_string();
    if input.is_empty() {
        return Task::none();
    }

    if app.active_session_id.as_deref() == Some(session_id.as_str()) {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.input_editor = iced::widget::text_editor::Content::with_text(&input);
        return send_current_session_text_only(app);
    }

    let previous_session_id = app.active_session_id.clone();
    let previous_chat = app.chat.clone();
    let previous_message_ids = app.chat_message_ids.clone();
    if let Some(previous_id) = previous_session_id.clone() {
        app.store_session_chat_snapshot(
            previous_id,
            crate::app::session::shared_chat_messages(previous_chat.clone()),
            previous_message_ids.clone(),
        );
    }

    app.active_session_id = Some(session_id.clone());
    restore_cached_chat_for_session(app, &session_id);
    app.invalidate_chat_ui_state();
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.input_editor = iced::widget::text_editor::Content::with_text(&input);

    let task = send_current_session_text_only(app);
    app.store_session_chat_snapshot(
        session_id,
        crate::app::session::shared_chat_messages(app.chat.clone()),
        app.chat_message_ids.clone(),
    );

    app.active_session_id = previous_session_id;
    app.chat = previous_chat;
    app.chat_message_ids = previous_message_ids;
    if let Some(previous_id) = app.active_session_id.clone() {
        app.store_session_chat_snapshot(
            previous_id,
            crate::app::session::shared_chat_messages(app.chat.clone()),
            app.chat_message_ids.clone(),
        );
    }
    app.invalidate_chat_ui_state();
    app.sync_active_session_from_chat();

    task
}

fn send_current_session_text_only(app: &mut App) -> Task<Message> {
    let previous_files = std::mem::take(&mut app.files);
    let task = send_current_session_input(app);
    app.files = previous_files;
    task
}

fn session_info_is_compacting(app: &App, session_id: &str) -> bool {
    app.sessions
        .iter()
        .chain(app.project_sessions.values().flatten())
        .find(|info| info.id == session_id)
        .is_some_and(|info| info.time.compacting.is_some())
}

fn set_session_compacting(app: &mut App, session_id: &str, compacting: Option<u64>) {
    for info in app
        .sessions
        .iter_mut()
        .chain(app.project_sessions.values_mut().flatten())
        .filter(|info| info.id == session_id)
    {
        info.time.compacting = compacting;
    }
}

fn request_session_compaction_confirmation(
    app: &mut App,
    session_id: String,
    project_path: String,
) -> Task<Message> {
    if session_id.trim().is_empty() {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "当前没有可压缩的会话",
            vec![],
        );
    }
    if app.session_is_requesting(&session_id) {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "请等待当前回复完成后再压缩会话",
            vec![],
        );
    }
    if session_info_is_compacting(app, &session_id) {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Info,
            "会话压缩正在进行中",
            vec![],
        );
    }

    app.show_session_actions_popover = false;
    app.session_menu_id = None;
    app.session_menu_anchor = None;
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    Task::perform(
        async move {
            let result =
                rfd::AsyncMessageDialog::new()
                    .set_level(rfd::MessageLevel::Warning)
                    .set_title(i18n.__("压缩当前会话"))
                    .set_description(i18n.__(
                        "压缩会生成摘要检查点以缩短模型上下文，原始聊天记录保持完整。确认继续吗？",
                    ))
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show()
                    .await;
            (session_id, project_path, matches!(result, rfd::MessageDialogResult::Yes))
        },
        |(session_id, project_path, confirmed)| {
            Message::Chat(ChatMessage::CompactSessionConfirmationResolved {
                session_id,
                project_path,
                confirmed,
            })
        },
    )
}

fn compact_session(
    app: &mut App,
    session_id: String,
    project_path: String,
    confirmed: bool,
) -> Task<Message> {
    if !confirmed {
        return Task::none();
    }
    if app.session_is_requesting(&session_id) {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "请等待当前回复完成后再压缩会话",
            vec![],
        );
    }
    if session_info_is_compacting(app, &session_id) {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Info,
            "会话压缩正在进行中",
            vec![],
        );
    }

    set_session_compacting(app, &session_id, Some(crate::app::time::now_ms()));
    Task::perform(
        async move {
            let result = async {
                let client = crate::app::gateway_client()?;
                let messages = client
                    .session_messages::<Vec<agent_message::WithParts>>(
                        &session_id,
                        Some(&project_path),
                    )
                    .await?;
                let message_id = latest_persisted_user_message_id(&messages)
                    .ok_or_else(|| "当前会话没有可用于压缩的用户消息".to_string())?;
                client
                    .session_summarize(
                        &session_id,
                        Some(&project_path),
                        &vw_gateway_client::GatewaySessionSummarizeBody { message_id },
                    )
                    .await
            }
            .await;
            (session_id, project_path, result)
        },
        |(session_id, project_path, result)| {
            Message::Chat(ChatMessage::CompactSessionFinished { session_id, project_path, result })
        },
    )
}

fn latest_persisted_user_message_id(messages: &[agent_message::WithParts]) -> Option<String> {
    messages
        .iter()
        .filter(|message| matches!(message.info, agent_message::Info::User(_)))
        .map(|message| message.info.id())
        .max()
        .map(str::to_string)
}

fn finish_session_compaction(
    app: &mut App,
    session_id: String,
    _project_path: String,
    result: Result<bool, String>,
) -> Task<Message> {
    set_session_compacting(app, &session_id, None);
    match result {
        Ok(true) => {
            app.show_localized_toast(crate::app::state::ToastKind::Success, "会话压缩完成", vec![])
        }
        Ok(false) => app.show_localized_toast(
            crate::app::state::ToastKind::Info,
            "当前会话没有需要压缩的较早历史",
            vec![],
        ),
        Err(error) => app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "会话压缩失败：%s",
            vec![format!("{}", error).into()],
        ),
    }
}

fn git_commit_selection_context(app: &App) -> String {
    let files = if app.staged_files_selected.is_empty() {
        "无".to_string()
    } else {
        app.staged_files_selected.join(", ")
    };
    let hunks = app
        .staged_hunks_selected
        .iter()
        .map(|(path, index)| format!("{path}#{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let new_lines = app
        .staged_lines_selected
        .iter()
        .map(|(path, line)| format!("{path}:{line}"))
        .collect::<Vec<_>>()
        .join(", ");
    let old_lines = app
        .staged_old_lines_selected
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

fn git_commit_generation_prompt(
    app: &App,
    project_path: &str,
    action: &QuickMessageGitCommitAction,
) -> String {
    let target = match action {
        QuickMessageGitCommitAction::StageAllAndCommit => {
            "当前工作树中的全部 Git 变更。请只读检查这些变更，不要执行暂存或提交。".to_string()
        }
        QuickMessageGitCommitAction::CommitSelected => format!(
            "用户在 Git 面板中选中的变更。请只读检查仓库差异，不要执行暂存或提交。\n\
             选中范围：\n{}",
            git_commit_selection_context(app)
        ),
    };
    format!(
        "请为以下 Git 变更生成提交信息。\n仓库路径：{project_path}\n目标：{target}\n\n\
         返回严格 JSON，禁止 Markdown 代码块或额外说明：\
         {{\"title\":\"简洁的提交标题\",\"summary\":\"说明主要改动与目的的摘要\"}}。\
         title 和 summary 都不能为空；title 可使用合适的 Conventional Commit 前缀。"
    )
}

fn generate_git_commit_message(
    app: &mut App,
    project_path: String,
    action: QuickMessageGitCommitAction,
) -> Task<Message> {
    if app.git_commit_in_progress {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "Git 提交正在进行中",
            vec![],
        );
    }
    if app.current_session_is_requesting() {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "请等待当前回复完成后再生成 Git 提交信息",
            vec![],
        );
    }
    if app.quick_message_handoff.is_some() {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "另一个快捷动作正在进行中",
            vec![],
        );
    }
    if app.current_session_runtime().task_mode_enabled {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "请先切换到聊天模式，再生成 Git 提交信息",
            vec![],
        );
    }
    if matches!(action, QuickMessageGitCommitAction::CommitSelected)
        && app.staged_files_selected.is_empty()
        && app.staged_hunks_selected.is_empty()
        && app.staged_lines_selected.is_empty()
        && app.staged_old_lines_selected.is_empty()
    {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "请先选择要提交的 Git 变更",
            vec![],
        );
    }

    let prompt = git_commit_generation_prompt(app, &project_path, &action);
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text(&prompt);
    let task = send_current_session_text_only(app);
    let Some(source_session_id) = app.active_session_id.clone() else {
        return task;
    };
    let request_id = app
        .get_session_runtime(&source_session_id)
        .active_agent_request
        .as_ref()
        .map(|request| request.id);
    let Some(request_id) = request_id else {
        return task;
    };

    app.quick_message_handoff = Some(QuickMessageHandoffState::WaitingForGitCommitMessage {
        request_id,
        source_session_id,
        project_path,
        action,
    });
    app.git_commit_in_progress = true;
    task
}

/// 当前活动会话是否由 plan-run 创建。
fn active_session_is_plan_run_child(app: &App) -> bool {
    let Some(session_id) = app.active_session_id.as_deref() else {
        return false;
    };
    app.get_session_runtime(session_id).is_plan_run_session
        || app
            .sessions
            .iter()
            .chain(app.project_sessions.values().flatten())
            .find(|session| session.id == session_id)
            .is_some_and(vw_shared::session::info::Info::is_plan_run)
}

fn check_agent_installation(app: &App, backend: String, query: Option<String>) -> Task<Message> {
    let session_id = app.active_session_id.clone();
    let agent = app.current_session_runtime().agent.clone();
    Task::perform(
        async { crate::app::gateway_client()?.global_acp_installed_get().await },
        move |result| {
            Message::Chat(ChatMessage::AgentInstallationChecked {
                session_id: session_id.clone(),
                agent: agent.clone(),
                backend: backend.clone(),
                query: query.clone(),
                result,
            })
        },
    )
}

fn send_current_session_input(app: &mut App) -> Task<Message> {
    let runtime = app.current_session_runtime();
    if let Some(backend) = runtime.acp_agent.clone() {
        return check_agent_installation(app, backend, Some(runtime.input_editor.text()));
    }
    send_current_session_input_checked(app)
}

fn send_current_session_input_checked(app: &mut App) -> Task<Message> {
    let runtime = app.current_session_runtime();
    let query = runtime.input_editor.text().to_string();
    let selected_tools = runtime.tool_selector.selected_manual_tools();
    let selected_skills = runtime.tool_selector.selected_manual_skills();
    let selected_shiju_skills = runtime.tool_selector.selected_shiju_skills();
    if query.trim().is_empty()
        && app.files.is_empty()
        && selected_tools.is_empty()
        && selected_skills.is_empty()
        && selected_shiju_skills.is_empty()
    {
        return Task::none();
    }
    let task_mode_enabled = runtime.task_mode_enabled;
    let task_mode_priority = runtime.task_mode_priority.clone();
    let task_mode_model = runtime.task_mode_model.clone();
    let task_mode_reasoning_effort = runtime.task_mode_reasoning_effort;
    let task_mode_task_split_mode = runtime.task_mode_task_split_mode;
    let task_mode_subtasks = runtime.task_mode_subtasks.clone();
    // plan-run 子会话（有 parent_id）续聊必须走普通 agent stream，避免误开 plan-run。
    // 工作流开关开启时始终进入 plan-run；目标模式底层固定开启，不可关闭。
    let workflow_mode_enabled =
        runtime.workflow_mode_enabled && !active_session_is_plan_run_child(app);
    let goal_mode_enabled = !active_session_is_plan_run_child(app);
    if let Some(command) = parse_inline_mode_command(&query) {
        return apply_inline_mode_command(app, command);
    }
    let attachments = std::mem::take(&mut app.files);
    let query = compose_query_with_session_context(
        &query,
        &selected_tools,
        &selected_skills,
        &selected_shiju_skills,
    );
    app.current_session_runtime_mut().tool_selector.clear_manual_context_selection();
    if task_mode_enabled {
        return Task::done(Message::TaskBoard(TaskBoardMessage::AddTaskFromInputWithOptions {
            content: compose_query_with_attachments(&query, &attachments),
            priority: task_mode_priority,
            model: task_mode_model,
            reasoning_effort: task_mode_reasoning_effort,
            task_split_mode: task_mode_task_split_mode,
            subtasks: task_mode_subtasks,
        }));
    }
    let runtime = app.current_session_runtime();
    let root = current_session_directory(app).or_else(|| app.project_path.clone());
    let acp_model_source = runtime.acp_model_source;
    let model = if runtime.auto_model
        || (runtime.acp_agent.is_some() && acp_model_source == AcpModelSource::CliLocal)
    {
        None
    } else {
        Some(runtime.model.clone())
    };
    let reasoning_effort = runtime.reasoning_effort;
    let acp_agent = runtime.acp_agent.clone();
    let allowed_tools = app.session_allowed_tools_for_request(&runtime);
    let agent = request_delegate_agent(runtime.agent.clone());
    let acp_test = acp_agent.is_some();
    let acp_allowed_tools = acp_test.then(|| allowed_tools.clone()).flatten();
    let acp_history_mode = runtime.acp_history_mode;
    let acp_recent_count = runtime.acp_recent_count.clamp(1, 20);
    let full_access_enabled = runtime.full_access_enabled;
    let acp_force_new_session = acp_agent.is_some()
        && (runtime.acp_rebuild_required
            || runtime.last_effective_acp_agent.as_ref() != acp_agent.as_ref());
    let created_ms = crate::app::time::now_ms();
    debug!(
        target: "vw_desktop",
        active_session_id = ?app.active_session_id,
        acp_test,
        acp_agent = ?acp_agent,
        acp_force_new_session,
        acp_history_mode = %acp_history_mode.as_str(),
        acp_recent_count,
        model = ?model,
        agent = ?agent,
        allowed_tools = ?allowed_tools,
        has_root = root.is_some(),
        query_len = query.len(),
        "desktop chat request prepared"
    );
    let item = QueueItem {
        created_ms,
        query,
        attachments,
        root,
        model,
        reasoning_effort,
        acp_test,
        acp_model_source,
        acp_agent,
        acp_allowed_tools,
        agent,
        allowed_tools,
        require_agent_tool: false,
        acp_force_new_session,
        acp_history_mode,
        acp_recent_count,
        full_access_enabled,
        send_behavior: app.chat_send_behavior,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled,
        goal_mode_enabled,
    };

    dispatch_queue_item(app, item)
}

/// 公开函数，执行 update 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub fn update(app: &mut App, message: ChatMessage) -> Task<Message> {
    if matches!(
        message,
        ChatMessage::AutoModelToggled(_)
            | ChatMessage::ModelSelected(_)
            | ChatMessage::ModelInputChanged(_)
            | ChatMessage::AcpModelSourceSelected(_)
    ) && app.current_session_has_started()
    {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Warning,
            "会话开始后不能切换模型",
            vec![],
        );
    }
    match message {
        ChatMessage::SendPressed => {
            if app.current_session_runtime().acp_source_pending.is_some()
                || app.current_session_runtime().acp_options_pending
            {
                return app.show_error_toast("模型来源正在保存，请稍后发送");
            }
            send_current_session_input(app)
        }
        ChatMessage::NewSessionProjectResolved { project_path, stream_id, pending, result } => {
            if app.project_path.as_deref() != Some(project_path.as_str())
                || app.agent_stream_id != stream_id
                || app.active_session_id.is_some()
            {
                return Task::none();
            }
            match result {
                Ok(project_id) => {
                    app.project_id = Some(project_id);
                    start(app, pending.item, pending.keep)
                }
                Err(error) => {
                    app.files.extend(pending.item.attachments);
                    app.push_localized_notification(
                        "解析项目失败，无法创建会话: %s",
                        vec![format!("{}", error).into()],
                    );
                    Task::none()
                }
            }
        }
        ChatMessage::SendToSession { session_id, input } => {
            send_to_existing_session(app, session_id, input)
        }
        ChatMessage::CompactSessionRequested { session_id, project_path } => {
            request_session_compaction_confirmation(app, session_id, project_path)
        }
        ChatMessage::CompactSessionConfirmationResolved { session_id, project_path, confirmed } => {
            compact_session(app, session_id, project_path, confirmed)
        }
        ChatMessage::CompactSessionFinished { session_id, project_path, result } => {
            finish_session_compaction(app, session_id, project_path, result)
        }
        ChatMessage::GenerateGitCommitMessage { project_path, action } => {
            generate_git_commit_message(app, project_path, action)
        }
        ChatMessage::CancelPressed => {
            let session_id = match &app.active_session_id {
                Some(id) => id.clone(),
                None => return Task::none(),
            };

            let is_requesting = app.current_session_is_requesting();
            if !is_requesting {
                return Task::none();
            }

            let request_id = app
                .get_session_runtime(&session_id)
                .active_agent_request
                .as_ref()
                .map(|request| request.id);
            if let Some(request_id) = request_id {
                crate::app::state::clear_pending_guide_handoff(request_id);
                clear_quick_message_handoff_for_request(app, request_id);
            }
            let stop_task = stream::stop_agent_stream(app, &session_id);
            if app.current_session_runtime().draft.is_some() {
                return stop_task;
            }
            let completed_ms = crate::app::time::now_ms();
            let elapsed = {
                let runtime = app.get_session_runtime_mut(&session_id);
                super::processing_timing::pause_runtime_processing_timers(runtime)
            };
            super::processing_timing::complete_tail_assistant_processing(
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
            let mut session = stream::load_session_or_default(app, session_id.clone());
            session.messages = app.chat.clone();
            session.message_ids = app.chat_message_ids.clone();
            session.updated_ms = completed_ms;
            let save_task = Task::batch([
                stop_task,
                save_session_task(session, stream::session_directory_for_save(app, &session_id)),
            ]);
            {
                let runtime = app.get_session_runtime_mut(&session_id);
                runtime.is_requesting = false;
                runtime.submit_anim = 0;
                runtime.active_agent_request = None;
            }
            app.sync_task_pet_from_runtime();
            app.show_send_mode_popover = false;
            app.finalize_stream_text();
            app.sync_active_session_from_chat();

            let runtime = app.get_session_runtime(&session_id);
            if runtime.queue.is_empty() {
                save_task
            } else {
                let runtime = app.get_session_runtime_mut(&session_id);
                let item = runtime.queue.remove(0);
                save_task.chain(start(app, item, true))
            }
        }
        ChatMessage::AcpModelSourceSelected(source) => {
            super::acp_options::select_source(app, source, None)
        }
        ChatMessage::AutoModelToggled(b) => {
            if app.current_session_runtime().acp_agent.is_some() {
                let model =
                    if b { "auto".to_string() } else { app.current_session_runtime().model };
                return super::acp_options::select_source(
                    app,
                    AcpModelSource::SessionModel,
                    Some(model),
                );
            }
            if let Some(session_id) = app.active_session_id.clone() {
                app.cancel_session_model_history_restore(&session_id);
            }
            let (model, auto_model) = {
                let runtime = app.current_session_runtime_mut();
                runtime.auto_model = b;
                if b {
                    runtime.task_mode_model = "auto".to_string();
                }
                (runtime.model.clone(), runtime.auto_model)
            };
            app.auto_model = auto_model;
            app.save_chat_model_preferences(model, auto_model)
        }
        ChatMessage::ReasoningEffortSelected(reasoning_effort) => {
            if app.current_session_runtime().acp_agent.is_some() {
                return Task::none();
            }
            app.current_session_runtime_mut().reasoning_effort = reasoning_effort;
            app.reasoning_effort = reasoning_effort;
            persist_session_reasoning_effort(app)
        }
        ChatMessage::AgentInstallationStatusesLoaded(result) => {
            if let Ok(statuses) = result {
                app.acp_settings.installed = statuses;
            }
            Task::none()
        }
        ChatMessage::AgentInstallationChecked { session_id, agent, backend, query, result } => {
            if app.active_session_id != session_id || app.current_session_runtime().agent != agent {
                return Task::none();
            }
            if app.current_session_runtime().acp_agent.as_deref() != Some(backend.as_str()) {
                return Task::none();
            }
            let installed = match result {
                Ok(statuses) => {
                    let installed = statuses.get(&backend).copied();
                    app.acp_settings.installed = statuses;
                    installed
                }
                Err(error) => {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "检测运行环境失败：%s",
                        vec![error.into()],
                    );
                }
            };
            if installed != Some(true) {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    if installed == Some(false) {
                        "CLI 未安装，请通过输入区的安装 / 运行设置安装"
                    } else {
                        "网关未提供该智能体的安装状态，请检查运行设置"
                    },
                    vec![],
                );
            }
            if query.is_some_and(|query| query == app.current_session_runtime().input_editor.text())
            {
                send_current_session_input_checked(app)
            } else {
                Task::none()
            }
        }
        ChatMessage::AgentRecommendedPromptSelected(index) => {
            app.show_recommended_prompt_menu = false;
            let runtime = app.current_session_runtime();
            if !runtime.input_editor.text().is_empty() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "请先处理现有草稿",
                    vec![],
                );
            }
            let key = runtime.agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
            let prompt = app
                .agents_settings
                .entries
                .iter()
                .find(|entry| entry.key == key)
                .and_then(|entry| entry.recommended_prompts.get(index))
                .cloned();
            if let Some(prompt) = prompt.filter(|prompt| !prompt.trim().is_empty()) {
                app.current_session_runtime_mut().input_editor =
                    iced::widget::text_editor::Content::with_text(&prompt);
                app.input_text = prompt;
            }
            Task::none()
        }
        ChatMessage::SessionAgentSelected(agent) => {
            app.show_recommended_prompt_menu = false;
            if app.current_session_has_started() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "会话开始后不能切换智能体",
                    vec![],
                );
            }
            let normalized_agent = normalize_delegate_agent(agent);
            let key = normalized_agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
            let Some(entry) = app
                .agents_settings
                .entries
                .iter()
                .find(|entry| entry.key == key && entry.can_start_session())
            else {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "智能体未启用或不支持当前入口",
                    vec![],
                );
            };
            let full_access = entry.defaults.full_access;
            let agent_label = Some(entry.label.clone());
            let acp_agent = match &entry.runtime {
                vw_config_types::agent::AgentRuntime::Native => None,
                vw_config_types::agent::AgentRuntime::Acp { backend } => Some(backend.clone()),
            };
            {
                let runtime = app.current_session_runtime_mut();
                if !runtime.full_access_overridden {
                    runtime.full_access_enabled = full_access;
                }
                runtime.full_access_initialized = true;
                runtime.agent = normalized_agent;
                runtime.agent_label = agent_label;
                runtime.acp_agent = acp_agent.clone();
                runtime.acp_options = Default::default();
                runtime.acp_options_pending = false;
                runtime.acp_model_source = Default::default();
                runtime.tool_selector.reset();
            }
            app.invalidate_project_chat_preferences_read();
            app.acp_agent = acp_agent.clone();
            app.show_acp_popover = false;
            app.show_reasoning_effort_popover = false;
            app.show_session_tool_selector_popover = false;
            if let Some(backend) = acp_agent {
                Task::batch([
                    check_agent_installation(app, backend, None),
                    super::acp_options::update(app, ChatMessage::AcpSourceRefresh),
                ])
            } else {
                Task::none()
            }
        }
        ChatMessage::AcpHistoryModeSelected(mode) => {
            let recent_count = {
                let runtime = app.current_session_runtime_mut();
                runtime.acp_history_mode = mode;
                runtime.acp_recent_count.clamp(1, 20)
            };
            persist_acp_history_preferences(app, mode, recent_count)
        }
        ChatMessage::AcpHistoryRecentCountChanged(raw) => {
            let parsed = raw.trim().parse::<usize>().ok().unwrap_or(3).clamp(1, 20);
            {
                let runtime = app.current_session_runtime_mut();
                runtime.acp_recent_count = parsed;
            }
            let mode = app.current_session_runtime().acp_history_mode;
            persist_acp_history_preferences(app, mode, parsed)
        }
        ChatMessage::ModelSelected(m) => {
            if app.current_session_runtime().acp_agent.is_some() {
                return super::acp_options::select_source(
                    app,
                    AcpModelSource::SessionModel,
                    Some(m),
                );
            }
            if let Some(session_id) = app.active_session_id.clone() {
                app.cancel_session_model_history_restore(&session_id);
            }
            let (model, auto_model): (String, bool) = {
                let runtime = app.current_session_runtime_mut();
                let trimmed = m.trim();
                runtime.model =
                    if trimmed.is_empty() { "auto".to_string() } else { trimmed.to_string() };
                runtime.auto_model = false;
                runtime.task_mode_model = runtime.model.clone();
                (runtime.model.clone(), runtime.auto_model)
            };
            let options = crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &model);
            let runtime = app.current_session_runtime_mut();
            let reset_effort =
                runtime.reasoning_effort.is_some() && !options.contains(&runtime.reasoning_effort);
            if reset_effort {
                runtime.reasoning_effort = None;
            }
            if runtime.task_mode_executor.is_none()
                && !options.contains(&runtime.task_mode_reasoning_effort)
            {
                runtime.task_mode_reasoning_effort = None;
            }
            app.model = model.clone();
            app.auto_model = auto_model;
            app.show_model_popover = false;
            app.show_mode_popover = false;
            app.reasoning_effort = app.current_session_runtime().reasoning_effort;
            let reasoning_task =
                if reset_effort { persist_session_reasoning_effort(app) } else { Task::none() };
            Task::batch([reasoning_task, app.save_chat_model_preferences(model, auto_model)])
        }
        ChatMessage::ModelInputChanged(m) => {
            if app.current_session_runtime().acp_agent.is_some() {
                return super::acp_options::select_source(
                    app,
                    AcpModelSource::SessionModel,
                    Some(m),
                );
            }
            if let Some(session_id) = app.active_session_id.clone() {
                app.cancel_session_model_history_restore(&session_id);
            }
            let (model, auto_model): (String, bool) = {
                let runtime = app.current_session_runtime_mut();
                let trimmed = m.trim();
                runtime.model =
                    if trimmed.is_empty() { "auto".to_string() } else { trimmed.to_string() };
                runtime.auto_model = runtime.model == "auto";
                runtime.task_mode_model = runtime.model.clone();
                (runtime.model.clone(), runtime.auto_model)
            };
            let options = crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &model);
            let runtime = app.current_session_runtime_mut();
            let reset_effort =
                runtime.reasoning_effort.is_some() && !options.contains(&runtime.reasoning_effort);
            if reset_effort {
                runtime.reasoning_effort = None;
            }
            if runtime.task_mode_executor.is_none()
                && !options.contains(&runtime.task_mode_reasoning_effort)
            {
                runtime.task_mode_reasoning_effort = None;
            }
            app.model = model.clone();
            app.auto_model = auto_model;
            app.reasoning_effort = app.current_session_runtime().reasoning_effort;
            let reasoning_task =
                if reset_effort { persist_session_reasoning_effort(app) } else { Task::none() };
            Task::batch([reasoning_task, app.save_chat_model_preferences(model, auto_model)])
        }
        ChatMessage::SessionToolBucketToggled(bucket) => {
            let base_tools = base_tools_for_selection(app);
            let runtime = app.current_session_runtime_mut();
            if !runtime.tool_selector.toggle_bucket(bucket) {
                app.push_localized_notification("至少保留一个工具分桶", vec![]);
            } else {
                runtime.tool_selector.reconcile_tools(&base_tools);
            }
            Task::none()
        }
        ChatMessage::SessionToolSelectorTabSelected(tab) => {
            {
                let runtime = app.current_session_runtime_mut();
                runtime.tool_selector.select_tab(tab);
            }
            if tab == SessionToolSelectorTab::Agent {
                let status = Task::perform(
                    async { crate::app::gateway_client()?.global_acp_installed_get().await },
                    |result| Message::Chat(ChatMessage::AgentInstallationStatusesLoaded(result)),
                );
                let profiles = if !app.agents_settings.dirty && !app.agents_settings.save_in_flight
                {
                    crate::app::message::settings::update(
                        app,
                        message::SettingsMessage::Agents(message::settings::AgentsMessage::Refresh),
                    )
                } else {
                    Task::none()
                };
                return Task::batch([status, profiles]);
            }
            if tab == SessionToolSelectorTab::Skills
                && app.skills_settings.catalog.is_empty()
                && !app.skills_settings.loading
            {
                Task::done(Message::Settings(message::SettingsMessage::SkillsRefresh))
            } else {
                Task::none()
            }
        }
        ChatMessage::OpenSessionToolSelectorTab(tab) => {
            let same_tab_open = app.show_session_tool_selector_popover
                && app.current_session_runtime().visible_session_tool_selector_tab() == tab;
            {
                let runtime = app.current_session_runtime_mut();
                runtime.tool_selector.select_tab(tab);
            }
            if same_tab_open {
                app.show_session_tool_selector_popover = false;
                return Task::none();
            }

            app.show_session_tool_selector_popover = true;
            app.show_model_popover = false;
            app.show_mode_popover = false;
            app.show_send_mode_popover = false;
            app.show_file_popover = false;
            app.show_acp_popover = false;
            app.show_reasoning_effort_popover = false;
            app.show_usage_popover = false;
            app.show_session_actions_popover = false;
            app.show_executor_popover = false;

            if tab == SessionToolSelectorTab::Agent {
                let status = Task::perform(
                    async { crate::app::gateway_client()?.global_acp_installed_get().await },
                    |result| Message::Chat(ChatMessage::AgentInstallationStatusesLoaded(result)),
                );
                let profiles = if !app.agents_settings.dirty && !app.agents_settings.save_in_flight
                {
                    crate::app::message::settings::update(
                        app,
                        message::SettingsMessage::Agents(message::settings::AgentsMessage::Refresh),
                    )
                } else {
                    Task::none()
                };
                return Task::batch([status, profiles]);
            }
            if tab == SessionToolSelectorTab::Skills
                && app.skills_settings.catalog.is_empty()
                && !app.skills_settings.loading
            {
                Task::done(Message::Settings(message::SettingsMessage::SkillsRefresh))
            } else {
                Task::none()
            }
        }
        ChatMessage::SessionToolSelectorSearchChanged(query) => {
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.set_query(query);
            Task::none()
        }
        ChatMessage::SessionToolSelectorSkillDirectoryScopeChanged(scope) => {
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.select_skill_directory_scope(scope);
            Task::none()
        }
        ChatMessage::SessionToolGroupCollapsedToggled(group) => {
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.toggle_group_collapsed(group);
            Task::none()
        }
        ChatMessage::SessionToolGroupToolsToggled(group) => {
            let base_tools = base_tools_for_selection(app);
            let runtime = app.current_session_runtime_mut();
            if !runtime.tool_selector.toggle_group_tools(&base_tools, group) {
                app.push_localized_notification("至少保留一个会话工具", vec![]);
            }
            Task::none()
        }
        ChatMessage::SessionToolSelectorSelectAll => {
            let base_tools = base_tools_for_selection(app);
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.select_all_tools(&base_tools);
            Task::none()
        }
        ChatMessage::SessionToolSelectorInvert => {
            let base_tools = base_tools_for_selection(app);
            let runtime = app.current_session_runtime_mut();
            if !runtime.tool_selector.invert_tools(&base_tools) {
                app.push_localized_notification("当前工具范围无法直接反选", vec![]);
            }
            Task::none()
        }
        ChatMessage::SessionToolToggled(tool_id) => {
            let base_tools = base_tools_for_selection(app);
            let runtime = app.current_session_runtime_mut();
            if !runtime.tool_selector.toggle_tool(&base_tools, &tool_id) {
                app.push_localized_notification("至少保留一个会话工具", vec![]);
            }
            Task::none()
        }
        ChatMessage::SessionManualToolSelected(tool_id) => {
            let tool_id = tool_id.trim().to_string();
            if tool_id.is_empty() {
                return Task::none();
            }
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.toggle_manual_tool(&tool_id);
            Task::none()
        }
        ChatMessage::SessionSkillSelected(skill_id) => {
            let skill_id = skill_id.trim().to_string();
            if skill_id.is_empty() {
                return Task::none();
            }
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.toggle_manual_skill(&skill_id);
            Task::none()
        }
        ChatMessage::SessionToolSelectorReset => {
            let base_tools = base_tools_for_selection(app);
            let runtime = app.current_session_runtime_mut();
            runtime.tool_selector.reset();
            runtime.tool_selector.reconcile_tools(&base_tools);
            Task::none()
        }
        ChatMessage::QueueRemove(i) => {
            let runtime = app.current_session_runtime_mut();
            if i < runtime.queue.len() {
                runtime.queue.remove(i);
            }
            Task::none()
        }
        ChatMessage::QueueEdit(i) => {
            let query = {
                let runtime = app.current_session_runtime_mut();
                if i >= runtime.queue.len() {
                    return Task::none();
                }
                runtime.queue.remove(i).query
            };
            app.current_session_runtime_mut().input_editor =
                iced::widget::text_editor::Content::with_text(&query);
            if app.active_session_id.is_none() {
                app.input_editor = app.current_session_runtime().input_editor;
            }
            iced::widget::operation::focus(app.input_editor_id.clone())
        }
        ChatMessage::QueueGuide(i) => {
            let item = {
                let runtime = app.current_session_runtime_mut();
                if i >= runtime.queue.len() {
                    return Task::none();
                }
                runtime.queue.remove(i)
            };
            let mut new_item = item;
            new_item.send_behavior = ChatSendBehavior::StopAndSend;
            dispatch_queue_item(app, new_item)
        }
        ChatMessage::QueueUp(i) => {
            let runtime = app.current_session_runtime_mut();
            if i > 0 && i < runtime.queue.len() {
                runtime.queue.swap(i, i - 1);
            }
            Task::none()
        }
        ChatMessage::QueueDown(i) => {
            let runtime = app.current_session_runtime_mut();
            if i + 1 < runtime.queue.len() {
                runtime.queue.swap(i, i + 1);
            }
            Task::none()
        }
        ChatMessage::QueueCopy(i) => {
            let query = {
                let runtime = app.current_session_runtime();
                match runtime.queue.get(i) {
                    Some(item) => item.query.clone(),
                    None => return Task::none(),
                }
            };
            if query.is_empty() {
                return Task::none();
            }
            Task::done(Message::CopyCode(query))
        }
        ChatMessage::SubmitTick => {
            let is_requesting = app.current_session_is_requesting();
            if !is_requesting {
                let runtime = app.current_session_runtime_mut();
                runtime.submit_anim = 0;
                return Task::none();
            }
            let runtime = app.current_session_runtime_mut();
            runtime.submit_anim = (runtime.submit_anim + 1) % 3;
            message::after(Duration::from_millis(200), Message::Chat(ChatMessage::SubmitTick))
        }
        ChatMessage::ForkSessionAt { anchor, target } => {
            if app.chat_fork_anchor.as_ref() != Some(&anchor) || !anchor.is_current(app) {
                return Task::none();
            }
            app.chat_fork_anchor = None;
            let endpoint = crate::app::gateway_client_endpoint();
            let show_reasoning_summary = app.dialogue_flow_show_reasoning_summary;
            Task::perform(
                async move {
                    let result = async {
                        use vw_gateway_client::{
                            GatewayClient, GatewaySessionForkBody, GatewaySessionForkTarget,
                        };

                        let client = GatewayClient::new(endpoint)?;
                        let destination = match target {
                            ForkSessionTarget::Local => GatewaySessionForkTarget::SameDirectory,
                            ForkSessionTarget::NewWorktree => {
                                GatewaySessionForkTarget::Worktree { name: None }
                            }
                        };
                        let info = client
                            .session_fork::<session::Info>(
                                &anchor.session_id,
                                Some(&anchor.directory),
                                &Some(GatewaySessionForkBody {
                                    before_turn_id: Some(anchor.user_message_id.clone()),
                                    target: Some(destination),
                                    ..Default::default()
                                }),
                            )
                            .await?;
                        let prepared =
                            crate::app::message::project::helpers::prepare_gateway_session_open(
                                &client,
                                &info.id,
                                &info.directory,
                                show_reasoning_summary,
                            )
                            .await?;
                        Ok(Box::new(ForkSessionLoaded { info, prepared }))
                    }
                    .await;
                    ChatMessage::ForkSessionFinished { anchor, target, result }
                },
                Message::Chat,
            )
        }
        ChatMessage::ForkSessionFinished { anchor, target, result } => {
            if !anchor.is_current(app) {
                return Task::none();
            }
            use crate::app::state::ToastKind;
            let ForkSessionLoaded { info, prepared } = match result {
                Ok(loaded) => *loaded,
                Err(error) => {
                    return app.show_localized_toast(
                        ToastKind::Error,
                        "分叉会话失败: %s",
                        vec![error.into()],
                    );
                }
            };
            let id = info.id.clone();
            let directory = info.directory.clone();
            let open = crate::app::message::project::open_project_session(
                app,
                directory,
                id,
                Some(prepared),
            );
            // Opening a different directory clears the current session list.
            crate::app::message::project::cache_external_session(app, info);
            let toast = app.show_localized_toast(
                ToastKind::Success,
                match target {
                    ForkSessionTarget::Local => "已分叉到新会话",
                    ForkSessionTarget::NewWorktree => "已分叉到新工作树会话",
                },
                vec![],
            );
            Task::batch([open, toast])
        }
        ChatMessage::ResetMessageIdsRecovered { session_id, msg_idx, result } => {
            if app.active_session_id.as_deref() != Some(session_id.as_str()) {
                return Task::none();
            }
            if let Ok(ids) = &result
                && (ids.len() == app.chat.len()
                    || message_id_for_index_in_ids(ids, msg_idx).is_some())
            {
                app.chat_message_ids.clone_from(ids);
                app.store_session_chat_snapshot(
                    session_id,
                    crate::app::session::shared_chat_messages(app.chat.clone()),
                    app.chat_message_ids.clone(),
                );
                return Task::done(Message::Chat(ChatMessage::ResetSessionToMessage { msg_idx }));
            }
            reset_session_locally(app, session_id, msg_idx)
        }
        ChatMessage::ResetSessionToMessage { msg_idx } => {
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            let runtime = app.current_session_runtime();
            if runtime.draft.is_some() {
                return if runtime.is_requesting {
                    Task::none()
                } else {
                    reset_session_locally(app, session_id, msg_idx)
                };
            }
            let Some(message_id) = message_id_for_index(app, msg_idx) else {
                let root = current_session_directory(app).or_else(|| app.project_path.clone());
                return recover_reset_message_ids_task(
                    gateway_endpoint(app),
                    session_id,
                    root,
                    msg_idx,
                );
            };
            let root = current_session_directory(app).or_else(|| app.project_path.clone());
            let retain_end = msg_idx.saturating_add(1);
            let retained_chat =
                app.chat.get(..retain_end).map(|slice| slice.to_vec()).unwrap_or_default();
            let retained_message_ids = app
                .chat_message_ids
                .get(..retain_end)
                .map(|slice| slice.to_vec())
                .unwrap_or_default();
            let save_task = reset_session_locally(app, session_id.clone(), msg_idx);
            let endpoint = gateway_endpoint(app);
            let gateway_task = Task::perform(
                async move {
                    let result = match vw_gateway_client::GatewayClient::new(
                        vw_gateway_client::GatewayEndpoint::new(endpoint.0, endpoint.1),
                    ) {
                        Ok(client) => {
                            client
                                .session_reset::<session::Info>(
                                    &session_id,
                                    root.as_deref(),
                                    &vw_gateway_client::GatewaySessionResetBody { message_id },
                                )
                                .await
                        }
                        Err(err) => Err(err),
                    };
                    ChatMessage::ResetSessionFinished {
                        result,
                        session_id,
                        retained_chat,
                        retained_message_ids,
                        directory: root,
                    }
                },
                Message::Chat,
            );
            Task::batch([save_task, gateway_task])
        }
        ChatMessage::ResetSessionFinished {
            result,
            session_id,
            retained_chat,
            retained_message_ids,
            directory,
        } => match result {
            Ok(info) => {
                if let Some(existing) =
                    app.sessions.iter_mut().find(|session| session.id == info.id)
                {
                    *existing = info.clone();
                }
                for sessions in app.project_sessions.values_mut() {
                    if let Some(existing) =
                        sessions.iter_mut().find(|session| session.id == info.id)
                    {
                        *existing = info.clone();
                    }
                }
                app.mark_session_acp_rebuild_required(&session_id);
                app.active_session_id = Some(session_id.clone());
                app.chat = retained_chat;
                app.chat_message_ids = retained_message_ids;
                app.chat_reset_menu_idx = None;
                app.invalidate_chat_ui_state();
                app.sync_active_session_preferences();
                app.rebuild_active_session_message_meta();
                app.store_session_chat_snapshot(
                    session_id.clone(),
                    crate::app::session::shared_chat_messages(app.chat.clone()),
                    app.chat_message_ids.clone(),
                );
                app.sync_active_session_from_chat();

                let now = crate::app::time::now_ms();
                let session_directory = directory.or_else(|| Some(info.directory.clone()));
                let local = models::ChatSession {
                    id: session_id,
                    title: info.title,
                    messages: app.chat.clone(),
                    message_ids: app.chat_message_ids.clone(),
                    calls: Vec::new(),
                    steps: Vec::new(),
                    created_ms: now,
                    updated_ms: now,
                };
                save_session_task(local, session_directory)
            }
            Err(error) => {
                app.push_localized_notification(
                    "重置会话失败: %s",
                    vec![format!("{}", error).into()],
                );
                Task::none()
            }
        },
        _ => Task::none(),
    }
}

/// 模块内可见函数，执行 start 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn start(app: &mut App, mut item: QueueItem, keep: bool) -> Task<Message> {
    app.agent_stream_id = app.agent_stream_id.wrapping_add(1);
    let stream_id = app.agent_stream_id;
    let mut save_tasks = Vec::new();
    let mut initial_save = Task::none();
    app.cache_active_session_chat();

    if app.active_session_id.is_none() {
        app.invalidate_project_chat_preferences_read();
        let id = id::descending(id::Prefix::Session, None)
            .unwrap_or_else(|_| format!("ses_{}", stream_id));
        let project_path = app.project_path.clone().unwrap_or_default();
        let inferred_project_id = app
            .project_id
            .clone()
            .or_else(|| {
                app.project_sessions
                    .get(&project_path)
                    .and_then(|sessions| sessions.first().map(|s| s.project_id.clone()))
            })
            .or_else(|| {
                app.sessions
                    .iter()
                    .find(|s| s.directory == project_path)
                    .map(|s| s.project_id.clone())
            });
        if inferred_project_id.is_none() {
            if project_path.trim().is_empty() {
                app.set_localized_input_error(
                    crate::app::i18n::input::ErrorField::General,
                    "未选择项目，无法创建会话",
                    vec![],
                );
                return Task::none();
            }
            // 项目页面已打开时，元数据可能尚未返回；由网关解析真实身份后继续发送。
            let directory = project_path.clone();
            return Task::perform(
                async move {
                    crate::app::message::project::helpers::resolve_gateway_project(&directory)
                        .await
                        .map(|project| project.id.0)
                },
                move |result| {
                    Message::Chat(ChatMessage::NewSessionProjectResolved {
                        project_path: project_path.clone(),
                        stream_id,
                        pending: Box::new(PendingSessionSend { item: item.clone(), keep }),
                        result,
                    })
                },
            );
        }
        let mut runtime = app.current_session_runtime();
        let directory = runtime
            .draft
            .as_ref()
            .filter(|draft| draft.project_path == project_path)
            .map_or_else(|| project_path.clone(), |draft| draft.directory.clone());
        runtime.draft = Some(SessionDraft { project_path, directory, history_len: app.chat.len() });
        app.session_runtime_states.insert(id.clone(), runtime);
        app.active_session_id = Some(id);
        save_tasks.push(persist_session_reasoning_effort(app));
    }

    let draft = app.current_session_runtime_ref().and_then(|runtime| runtime.draft.clone());
    let is_draft = draft.is_some();
    if let Some(draft) = draft {
        item.root = Some(draft.directory);
        if app.chat.len() > draft.history_len {
            app.chat.truncate(draft.history_len);
            app.chat_message_ids.truncate(draft.history_len);
            app.active_session_view_state.steps.clear();
            app.invalidate_chat_ui_state();
        }
    }
    let request_history = item.request_history_override.clone().unwrap_or_else(|| app.chat.clone());
    let request_query = if item.resume_history_only {
        String::new()
    } else {
        compose_query_with_attachments(&item.query, &item.attachments)
    };
    let message_model = item.model.clone().and_then(|model| {
        let model = model.trim();
        (!model.is_empty()).then(|| model.to_string())
    });
    let request_started_ms = crate::app::time::now_ms();
    let appended_start_idx = app.chat.len();
    let appended_user_idx = if request_query.trim().is_empty() {
        None
    } else {
        app.chat.push(models::ChatMessage {
            role: models::ChatRole::User,
            content: request_query.clone(),
            model: message_model.clone(),
            think_timing: Vec::new(),
            processing_timing: None,
        });
        app.chat_message_ids.push(None);
        Some(appended_start_idx)
    };
    app.chat.push(models::ChatMessage {
        role: models::ChatRole::Assistant,
        content: String::new(),
        model: message_model,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    super::processing_timing::start_tail_assistant_processing(&mut app.chat, request_started_ms);
    app.chat_message_ids.push(None);
    app.invalidate_chat_ui_for_message_idx(
        appended_start_idx.min(app.chat.len().saturating_sub(1)),
    );
    if let Some(last_idx) = app.chat.len().checked_sub(1) {
        app.invalidate_chat_ui_for_message_idx(last_idx);
    }
    app.sync_chat_message_estimated_heights_len();
    app.refine_chat_message_estimated_heights(appended_start_idx, app.chat.len());
    app.active_session_view_state.updated_ms = item.created_ms;
    app.rebuild_active_session_message_meta();
    if let Some(session_id) = app.active_session_id.clone() {
        app.store_session_chat_snapshot(
            session_id,
            crate::app::session::shared_chat_messages(app.chat.clone()),
            app.chat_message_ids.clone(),
        );
    }
    if !keep {
        let is_empty_session = app.active_session_id.is_none();
        let runtime = app.current_session_runtime_mut();
        runtime.input_editor = iced::widget::text_editor::Content::new();
        if is_empty_session {
            app.input_editor = runtime.input_editor.clone();
        }
    }

    if let Some(session_id) = app.active_session_id.clone() {
        let runtime = app.get_session_runtime_mut(&session_id);
        runtime.is_requesting = true;
        runtime.request_started_ms = request_started_ms;
        runtime.processing_timer.reset();
        runtime.processing_timer.start_at(web_time::Instant::now());
        runtime.llm_timer.reset();
        runtime.submit_anim = 0;
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
        runtime.active_agent_request = Some(Arc::new(AgentRequest {
            id: stream_id,
            session: session_id.clone(),
            query: request_query,
            root: item.root,
            model: item.model,
            reasoning_effort: item.reasoning_effort,
            acp_test: item.acp_test,
            acp_model_source: item.acp_model_source,
            acp_agent: item.acp_agent,
            acp_allowed_tools: item.acp_allowed_tools,
            agent: item.agent,
            allowed_tools: item.allowed_tools,
            require_agent_tool: item.require_agent_tool,
            acp_force_new_session: item.acp_force_new_session,
            acp_history_mode: item.acp_history_mode,
            acp_recent_count: item.acp_recent_count,
            full_access_enabled: item.full_access_enabled,
            resume_history_only: item.resume_history_only,
            workflow_mode_enabled: item.workflow_mode_enabled,
            goal_mode_enabled: item.goal_mode_enabled,
            history: request_history,
        }));
        if !is_draft
            && runtime
                .queue
                .first()
                .is_some_and(|queued| queued.send_behavior == ChatSendBehavior::Guide)
        {
            crate::app::state::mark_pending_guide_handoff(stream_id);
        } else {
            crate::app::state::clear_pending_guide_handoff(stream_id);
        }
        if runtime.active_agent_request.as_ref().is_some_and(|request| request.acp_test) {
            runtime.acp_rebuild_required = false;
        }
        runtime.last_effective_acp_agent =
            runtime.active_agent_request.as_ref().and_then(|request| request.acp_agent.clone());
        app.sync_task_pet_from_runtime();

        if !is_draft {
            let now = crate::app::time::now_ms();
            let title = app
                .sessions
                .iter()
                .find(|s| s.id == session_id)
                .map(|s| s.title.clone())
                .unwrap_or_else(|| "新会话".to_string());
            let mut local = models::ChatSession {
                id: session_id.clone(),
                title,
                messages: vec![],
                message_ids: vec![],
                calls: vec![],
                steps: vec![],
                created_ms: now,
                updated_ms: now,
            };
            local.messages = app.chat.clone();
            local.message_ids = app.chat_message_ids.clone();
            local.updated_ms = now;
            let session_directory =
                current_session_directory(app).or_else(|| app.project_path.clone());
            // A bounded history page is not a complete replacement snapshot.
            if app
                .session_runtime_states
                .get(&session_id)
                .is_none_or(|runtime| runtime.stream_save_seq_base == 0)
            {
                // Persist the initial rows of a registered session before starting deltas.
                initial_save = save_session_task(local, session_directory);
            }
            // The snapshot writer can fail or be coalesced independently. Include the
            // user row and assistant placeholder in the acknowledged delta queue too.
            app.get_session_runtime_mut(&session_id).stream_save_failures = 0;
            save_tasks.push(stream::register_stream_session_messages(
                app,
                &session_id,
                appended_start_idx,
                app.chat.len(),
                now,
            ));
        }
    }

    app.sync_active_session_from_chat();
    let submit = message::after(Duration::from_millis(200), Message::Chat(ChatMessage::SubmitTick));
    save_tasks.push(submit);
    save_tasks.push(Task::done(Message::Chat(ChatMessage::LoadInputPanelTodos)));
    if let Some(user_idx) = appended_user_idx {
        if user_idx == 0 {
            // 首轮消息天然位于顶部。此时滚动容器刚从空会话视图挂载，
            // 避免在首个有效视口回执前执行 snap_to 和布局测量。
            super::focus_chat_on_turn(app, user_idx);
        } else {
            save_tasks.push(super::scroll_chat_to_turn_top_task(app, user_idx));
        }
    } else if app.chat_auto_scroll {
        save_tasks.push(super::scroll_chat_to_bottom_task(app));
    }
    initial_save.chain(Task::batch(save_tasks))
}
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
