//! 当前聊天和项目的会话选择、恢复与归档；所有权校验先于绑定替换。
use super::*;
use channel_support::routing_context::set_route_selection;
use super::session_commands::{effort_levels, handle_clear_session, handle_new_session};
use super::task_mode::set_sender_task_mode;

/// 持久化当前已登记会话的模型选项，不为旧映射推断所有权。
pub use crate::manager::persist_current_route;

#[derive(Clone)]
struct ArchiveConfirmation {
    id: String,
    metadata: String,
}

fn archive_confirmations() -> &'static Mutex<HashMap<ChannelMessageScope, ArchiveConfirmation>> {
    static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, ArchiveConfirmation>>> =
        OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn invalidate_archive(snapshot: &ChannelMessageSnapshot) {
    archive_confirmations().lock().unwrap_or_else(|e| e.into_inner()).remove(&snapshot.scope);
    super::history_commands::invalidate_history(snapshot);
}

pub use crate::manager::is_session_lifecycle_command;

/// 在入站读锁之前取得聊天写锁，整个校验和提交期间拒绝新模型运行。
pub async fn handle_session_lifecycle_command(
    host: &dyn crate::command_host::CommandHost,
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    command: ChannelRuntimeCommand,
) -> String {
    let gate = channel_chat_gate(ctx, msg);
    let Ok(_guard) = gate.try_write() else {
        return "当前聊天繁忙，会话保持不变。".into();
    };
    let mut scoped = ctx.clone();
    scoped.message_snapshot = None;
    let result = async {
        let snapshot = resolve_channel_message_snapshot(&scoped, msg).await?;
        if let Some(previous) = &ctx.message_snapshot {
            anyhow::ensure!(previous.scope == snapshot.scope, "项目状态已变化，请重试");
        }
        scoped.workspace_dir = Arc::new(snapshot.directory.clone());
        scoped.message_snapshot = Some(snapshot.clone());
        let mut result = None;
        host.in_session_scope(&snapshot, Box::pin(async {
            result = Some(session_lifecycle_command(host, &scoped, msg, command).await);
        })).await;
        result.expect("session scope completed")
    }
    .await;
    match result {
        Ok(response) => response,
        Err(err) => format!(
            "会话操作失败：{}。当前绑定保持不变；使用 /sessions 查看候选。",
            agent_providers::sanitize_api_error(&err.to_string())
        ),
    }
}

async fn session_lifecycle_command(
    host: &dyn crate::command_host::CommandHost,
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    command: ChannelRuntimeCommand,
) -> Result<String> {
    let snapshot = ctx.message_snapshot.as_ref().context("缺少消息作用域")?;
    let key = snapshot.scope.key();
    let current_id = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned();
    if let Some(id) = &current_id {
        anyhow::ensure!(
            matches!(host.session_activity(id), crate::command_host::SessionActivity::Idle),
            "当前会话繁忙"
        );
    }
    if matches!(&command, ChannelRuntimeCommand::ListSessions)
        || matches!(&command, ChannelRuntimeCommand::ResumeSession(id) if id.is_empty())
    {
        let sessions = owned_channel_sessions(host, snapshot).await?;
        let mut response = String::from("当前聊天和项目的会话（/resume <ID> 恢复）：\n");
        for info in sessions.iter().take(20) {
            let title = if info.title.is_empty() {
                channel_session_preview(host, snapshot, &info.id).await?
            } else {
                info.title.clone()
            };
            let title: String = title.chars().filter(|ch| !ch.is_control()).take(60).collect();
            response.push_str(&format!("{}  {}\n", info.id, title));
        }
        if sessions.is_empty() {
            response.push_str("暂无可恢复会话。使用 /new 创建。\n");
        }
        if sessions.len() > 20 {
            response.push_str("最多显示 20 项，可使用已知 ID 恢复。\n");
        }
        return Ok(response);
    }
    match command {
        ChannelRuntimeCommand::NewSession => Ok(Box::pin(handle_new_session(host, ctx, msg, &key)).await),
        ChannelRuntimeCommand::ClearSession => {
            persist_current_route(ctx, &get_route_selection(ctx, &key)).await?;
            let response = handle_clear_session(ctx, msg, &key).await;
            invalidate_archive(snapshot);
            Ok(response)
        }
        ChannelRuntimeCommand::ResumeSession(id) => {
            restore_channel_session(host, ctx, snapshot, &id).await
        }
        ChannelRuntimeCommand::ContinueSession => {
            let sessions = owned_channel_sessions(host, snapshot).await?;
            let recent = sessions.first().context("暂无可继续的非归档会话")?;
            restore_channel_session(host, ctx, snapshot, &recent.id).await
        }
        ChannelRuntimeCommand::RenameSession(title) => {
            let id = current_id.context("尚未创建会话")?;
            channel_session_ownership(snapshot, &id).await?;
            let info = host.get_session(&id).await?;
            validate_session(snapshot, &info)?;
            anyhow::ensure!(!info.is_first_response_pending(), "首轮回复完成后才能重命名");
            let title = title.trim();
            anyhow::ensure!(
                !title.is_empty()
                    && title.chars().count() <= 100
                    && !title.chars().any(char::is_control),
                "标题须为 1–100 个非控制字符"
            );
            host.set_session_title(&id, title).await?;
            invalidate_archive(snapshot);
            Ok(format!("会话 {id} 已重命名为 {title}。"))
        }
        ChannelRuntimeCommand::ArchiveSession(args) => {
            let id = current_id.context("尚未创建会话")?;
            channel_session_ownership(snapshot, &id).await?;
            let info = host.get_session(&id).await?;
            validate_session(snapshot, &info)?;
            if args.is_empty() {
                archive_confirmations().lock().unwrap_or_else(|e| e.into_inner()).insert(
                    snapshot.scope.clone(),
                    ArchiveConfirmation { id: id.clone(), metadata: serde_json::to_string(&info)? },
                );
                return Ok(format!("归档保留历史。确认请输入 /archive confirm {id}"));
            }
            anyhow::ensure!(args == format!("confirm {id}"), "确认必须匹配当前会话 ID");
            let pending = archive_confirmations()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&snapshot.scope)
                .cloned()
                .context("请先使用 /archive 请求确认")?;
            anyhow::ensure!(
                pending.id == id && pending.metadata == serde_json::to_string(&info)?,
                "会话状态已变化，请重新确认"
            );
            persist_current_route(ctx, &get_route_selection(ctx, &key)).await?;
            host.set_session_archived(&id, Some(host.command_now_ms())).await?;
            sender_session_store()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&snapshot.scope);
            clear_sender_history(ctx, &key);
            set_sender_task_mode(ctx, &key, false);
            invalidate_archive(snapshot);
            Ok(format!("会话 {id} 已归档，历史已保留。"))
        }
        command @ (ChannelRuntimeCommand::ForkSession(_)
        | ChannelRuntimeCommand::CompactSession(_)
        | ChannelRuntimeCommand::ResetSession(_)) => {
            super::history_commands::handle_history_command(host, ctx, command).await
        }
        _ => anyhow::bail!("未知会话命令"),
    }
}

fn validate_session(
    snapshot: &ChannelMessageSnapshot,
    info: &session_info::Info,
) -> Result<()> {
    anyhow::ensure!(
        info.project_id == snapshot.scope.project
            && Path::new(&info.directory) == snapshot.directory
            && info.time.archived.is_none(),
        "会话项目不匹配、目录已变化或会话已归档"
    );
    Ok(())
}

async fn restore_channel_session(
    host: &dyn crate::command_host::CommandHost,
    ctx: &ChannelRuntimeContext,
    snapshot: &ChannelMessageSnapshot,
    id: &str,
) -> Result<String> {
    let ownership = channel_session_ownership(snapshot, id).await?;
    let info = host.get_session(id).await?;
    validate_session(snapshot, &info)?;
    anyhow::ensure!(!info.is_first_response_pending(), "首轮回复尚未完成");
    anyhow::ensure!(
        matches!(host.session_activity(id), crate::command_host::SessionActivity::Idle),
        "目标会话繁忙"
    );
    let mut route = ownership.route.unwrap_or(ChannelRouteSelection {
        provider: ctx.default_provider.as_ref().clone(),
        model: ctx.model.as_ref().clone(),
        reasoning_effort: None,
        task_mode_enabled: false,
    });
    route.task_mode_enabled = false;
    let model = Box::pin(provider::get_model(&route.provider, &route.model))
        .await
        .context("保存的模型已不可用")?;
    anyhow::ensure!(model.status != "deprecated", "保存的模型已停用");
    if let Some(effort) = &route.reasoning_effort {
        anyhow::ensure!(effort_levels(&model).contains(effort), "保存的推理强度已不可用");
    }
    let history = host.load_ui_session(snapshot, id).await?;
    let history = history.map(|session| session.messages).unwrap_or_default();
    let mut history: Vec<ChatMessage> = history
        .into_iter()
        .map(|message| {
            let role = match message.role {
                session_port::ui_types::ChatRole::User => "user",
                session_port::ui_types::ChatRole::Assistant => "assistant",
                session_port::ui_types::ChatRole::System => "system",
                session_port::ui_types::ChatRole::Developer => "developer",
                session_port::ui_types::ChatRole::Tool => "tool",
            };
            ChatMessage { role: role.into(), content: message.content }
        })
        .collect();
    let persisted = session_engine::message::messages(id, None).await?;
    if !persisted.is_empty() {
        history = super::history_commands::cached_history(persisted);
    }
    let key = snapshot.scope.key();
    persist_current_route(ctx, &get_route_selection(ctx, &key)).await?;
    // 全部读取/校验完成后无 await：同时替换历史、模型选项和当前绑定。
    ctx.conversation_histories
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key.clone(), history);
    set_route_selection(ctx, &key, route);
    sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(snapshot.scope.clone(), id.into());
    invalidate_archive(snapshot);
    Ok(format!("已恢复会话 {id}。"))
}

async fn channel_session_preview(host: &dyn crate::command_host::CommandHost, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<String> {
    #[cfg(target_arch = "wasm32")]
    { let _ = (host, snapshot, id); Ok(String::new()) }
    #[cfg(not(target_arch = "wasm32"))]
    { Ok(host.load_session_preview(snapshot, id).await?.unwrap_or_else(|| "（附件）".into())) }
}
