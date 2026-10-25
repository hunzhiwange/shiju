//! 通道会话映射与 processor 集成；映射绑定完整消息作用域。
use super::*;

pub(crate) fn sender_session_store() -> &'static Mutex<HashMap<ChannelMessageScope, String>> {
    static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) async fn clear_sender_session_id(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> Result<()> {
    let snapshot = resolve_channel_message_snapshot(ctx, msg).await?;
    sender_session_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&snapshot.scope);
    Ok(())
}

/// 为消息生成初始会话标题
///
/// 从消息内容提取并格式化一个简短的标题，用于新会话的初始标识。
/// 标题会进行规范化处理（去除多余空格）并限制长度。
///
/// # 参数
///
/// - `msg`：通道消息，包含内容和通道/发送者信息
///
/// # 返回值
///
/// 返回生成的标题字符串：
/// - 如果消息有内容：返回前 50 个字符（超出时添加 "..."）
/// - 如果消息内容为空：返回 `{channel} {sender}` 格式的标题
///
/// # 处理规则
///
/// 1. 规范化空白字符（将连续空格合并为单个空格）
/// 2. 去除首尾空白
/// 3. 限制最多 50 个字符
/// 4. 如果截断，在末尾添加 "..."
pub(crate) fn initial_session_title_for_message(msg: &traits::ChannelMessage) -> String {
    let normalized = msg.content.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return format!("{} {}", msg.channel, msg.sender);
    }
    let mut chars = trimmed.chars();
    let title: String = chars.by_ref().take(50).collect();
    if chars.next().is_some() { format!("{}...", title.trim_end()) } else { title }
}

/// 异步生成并刷新会话标题
///
/// 在后台任务中异步生成优化的会话标题，并更新到会话记录中。
/// 这允许系统使用 AI 模型根据消息内容生成更有意义的标题，
/// 同时不阻塞消息处理流程。
///
/// # 参数
///
/// - `session_id`：需要更新标题的会话 ID
/// - `first_user_content`：用户的初始消息内容，用于生成标题
/// - `preferred_model`：可选的首选模型标识符，用于标题生成
///
/// # 行为说明
///
/// 该函数立即返回，实际工作在异步任务中执行：
/// 1. 调用 `title::generate_from_content` 生成新标题
/// 2. 如果生成成功，更新会话的 title 字段
/// 3. 忽略任何错误（后台任务失败不应影响主流程）
pub(crate) fn spawn_channel_session_title_refresh(
    session_id: String,
    first_user_content: String,
    preferred_model: Option<String>,
) {
    let tenant = crate::session::ui_store::current_tenant_scope();
    tokio::spawn(crate::session::ui_store::with_tenant_scope(tenant, async move {
        let generated = crate::app::agent::session::title::generate_from_content(
            session_id.clone(),
            first_user_content,
            preferred_model,
            None,
        )
        .await;
        let Ok(title) = generated else { return };
        let _ =
            crate::app::agent::session::session::update_any(&session_id, |s| s.title = title).await;
    }));
}

/// 会话创建失败返回错误，不缓存伪造的会话 ID。
pub(crate) async fn resolve_or_create_sender_session_id(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
) -> Result<String> {
    anyhow::ensure!(
        snapshot.scope.chat == {
            let mut chat = ChannelChatScope::from_message(msg);
            chat.tenant = snapshot.scope.chat.tenant.clone();
            chat
        },
        "channel message snapshot identity mismatch"
    );
    anyhow::ensure!(snapshot.directory.is_dir(), "channel project directory unavailable");
    crate::session::ui_store::with_tenant_scope(snapshot.scope.chat.tenant.clone(), async {
        let existing = sender_session_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&snapshot.scope)
            .cloned();
        if let Some(existing) = existing {
            if let Ok(info) = crate::session::session::get_any(&existing).await {
                if info.project_id == snapshot.scope.project
                    && info.time.archived.is_none()
                    && Path::new(&info.directory) == snapshot.directory
                {
                    return Ok(existing);
                }
            }
        }
        create_channel_session(snapshot, msg, None).await
    })
    .await
}

/// 所有权只随新建登记；作用域键哈希避免把外部聊天 ID 当作文件路径。
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ChannelSessionOwnership {
    pub(crate) scope: String,
    pub(crate) route: Option<ChannelRouteSelection>,
}

pub(super) fn ownership_scope_key(snapshot: &ChannelMessageSnapshot) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(snapshot.scope.key().as_bytes()))
}

pub(crate) async fn channel_session_ownership(
    snapshot: &ChannelMessageSnapshot,
    id: &str,
) -> Result<ChannelSessionOwnership> {
    anyhow::ensure!(
        id.starts_with("ses_") && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
        "无效会话 ID"
    );
    let record: ChannelSessionOwnership =
        crate::storage::read(&["channel_session", &ownership_scope_key(snapshot), id])
            .await
            .context("会话不属于当前聊天和项目")?;
    anyhow::ensure!(record.scope == snapshot.scope.key(), "会话作用域不匹配");
    Ok(record)
}

pub(crate) async fn save_channel_session_route(
    snapshot: &ChannelMessageSnapshot,
    id: &str,
    route: &ChannelRouteSelection,
) -> Result<()> {
    let mut record = channel_session_ownership(snapshot, id).await?;
    record.route = Some(route.clone());
    crate::storage::write(&["channel_session", &ownership_scope_key(snapshot), id], &record)
        .await?;
    Ok(())
}

pub(crate) async fn owned_channel_sessions(
    snapshot: &ChannelMessageSnapshot,
) -> Result<Vec<crate::session::session::Info>> {
    let mut sessions = Vec::new();
    for key in crate::storage::list(&["channel_session", &ownership_scope_key(snapshot)]).await? {
        let Some(id) = key.last() else { continue };
        channel_session_ownership(snapshot, id).await?;
        match crate::session::session::get_any(id).await {
            Ok(info)
                if info.project_id == snapshot.scope.project
                    && Path::new(&info.directory) == snapshot.directory
                    && info.time.archived.is_none() =>
            {
                sessions.push(info)
            }
            Ok(_)
            | Err(crate::session::session::Error::Storage(crate::storage::Error::NotFound(_))) => {}
            Err(err) => return Err(err.into()),
        }
    }
    sessions.sort_by(|a, b| b.time.updated.cmp(&a.time.updated).then(a.id.cmp(&b.id)));
    Ok(sessions)
}

/// 会话与所有权均写入成功后才替换当前映射。
pub(crate) async fn create_channel_session(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
    route: Option<ChannelRouteSelection>,
) -> Result<String> {
    anyhow::ensure!(!cfg!(target_arch = "wasm32"), "通道会话所有权持久化不支持 WASM");
    anyhow::ensure!(snapshot.directory.is_dir(), "channel project directory unavailable");
    let directory = snapshot.directory.to_string_lossy().to_string();
    let context = crate::project::instance::Context {
        directory: directory.clone(),
        worktree: snapshot.project.worktree.clone(),
        project: snapshot.project.clone(),
    };
    let info = crate::project::instance::provide_context(context, async move {
        crate::session::session::create_next(crate::session::session::CreateInput {
            parent_id: None,
            title: Some(initial_session_title_for_message(msg)),
            directory,
            permission: None,
        })
        .await
    })
    .await
    .context("failed to create channel session")?;
    let record = ChannelSessionOwnership { scope: snapshot.scope.key(), route };
    if let Err(err) = crate::storage::write(
        &["channel_session", &ownership_scope_key(snapshot), &info.id],
        &record,
    )
    .await
    {
        // 登记失败的孤立新会话不可恢复；旧绑定未动。
        let _ = crate::session::session::remove_any(&info.id).await;
        return Err(err.into());
    }
    sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(snapshot.scope.clone(), info.id.clone());
    Ok(info.id)
}

/// 异步为通道运行会话处理器
///
/// 启动会话处理器并处理其产生的流式事件，将增量内容收集并可选地转发到外部通道。
/// 这是将底层会话处理器与通道集成的主要接口。
///
/// # 参数
///
/// - `req`：会话处理器请求，包含执行所需的所有信息
/// - `delta_tx`：可选的增量内容发送器，用于实时转发输出内容
///
/// # 返回值
///
/// - `Ok(String)`：返回完整的输出内容
/// - `Err(anyhow::Error)`：处理器出错或异常退出
///
/// # 事件处理
///
/// 处理以下流式事件：
/// - `Delta`：增量内容，追加到输出并通过 `delta_tx` 转发（如果提供）
/// - `Done`：处理完成，返回累积的输出
/// - `Error`：处理错误，立即返回错误
/// - `StepStart` / `StepFinish`：步骤事件，当前被忽略
///
/// # 并发模型
///
/// 使用 `spawn_blocking` 在独立线程中运行同步的会话处理器，
/// 通过无界通道异步接收事件
pub(crate) async fn run_session_processor_for_channel(
    req: crate::app::agent::session::processor::Request,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
) -> anyhow::Result<String> {
    let mut req = req;
    if req.persist_app_session_artifacts && req.app_session_scope.is_none() {
        req.app_session_scope =
            crate::app::agent::session::processor::app_session_scope_from_root_async(
                req.root.as_deref(),
            )
            .await;
    }

    // 创建无界通道用于事件传递
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<
        crate::app::agent::session::processor::StreamEvent,
    >();

    // 阻塞线程不会继承 task-local 项目上下文；按请求目录和租户重新绑定，
    // 否则项目专属模型在下一条聊天中会退回全局目录查询。
    let tenant = crate::session::ui_store::current_tenant_scope();
    let instance_context = if let Some(directory) = req.root.as_deref() {
        let (project, worktree) =
            crate::project::from_directory(&tenant, Path::new(directory)).await?;
        if let Some(scope) = &req.app_session_scope {
            anyhow::ensure!(scope == &project.id, "channel processor project scope mismatch");
        }
        Some(crate::project::instance::Context { directory: directory.into(), worktree, project })
    } else {
        None
    };
    let run_processor = move || {
        block_on_future(crate::session::ui_store::with_tenant_scope(tenant, async move {
            let run = async move {
                crate::app::agent::session::processor::run(req, move |ev| {
                    event_tx.send(ev).is_ok()
                });
            };
            if let Some(context) = instance_context {
                crate::project::instance::provide_context(context, run).await;
            } else {
                run.await;
            }
        }));
    };
    #[cfg(test)]
    let run_processor = crate::global_database::inherit_test_database_context(run_processor);
    tokio::task::spawn_blocking(run_processor);

    // 处理流式事件
    let mut output = String::new();
    while let Some(ev) = event_rx.recv().await {
        match ev {
            crate::app::agent::session::processor::StreamEvent::LlmRawArtifact(_) => {}
            crate::app::agent::session::processor::StreamEvent::Delta(delta) => {
                output.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            crate::app::agent::session::processor::StreamEvent::Tool(event) => {
                let delta = format!("tool {}\n{}\n", event.name, event.payload);
                output.push_str(&delta);
                if let Some(tx) = delta_tx.as_ref() {
                    let _ = tx.send(delta).await;
                }
            }
            crate::app::agent::session::processor::StreamEvent::Done(_) => return Ok(output),
            crate::app::agent::session::processor::StreamEvent::Error(err) => {
                anyhow::bail!(err)
            }
            crate::app::agent::session::processor::StreamEvent::StepStart { .. }
            | crate::app::agent::session::processor::StreamEvent::StepFinish { .. }
            | crate::app::agent::session::processor::StreamEvent::PostToolRound { .. }
            | crate::app::agent::session::processor::StreamEvent::Compaction { .. }
            | crate::app::agent::session::processor::StreamEvent::ContextLoaded { .. } => {}
        }
    }

    // 如果没有收到终止事件，视为异常
    anyhow::bail!("session processor exited without terminal event")
}

/// 将通道聊天消息转换为会话历史格式
///
/// 将通道模块的 `ChatMessage` 转换为模型层的 `ChatMessage` 格式，
/// 用于传递给会话处理器或 AI 模型。
///
/// # 参数
///
/// - `turns`：通道聊天消息切片，包含对话轮次
///
/// # 返回值
///
/// 返回转换后的模型层消息向量 `Vec<crate::session::ui_types::ChatMessage>`
///
/// # 角色映射
///
/// - `"user"` -> `ChatRole::User`
/// - `"assistant"` -> `ChatRole::Assistant`
/// - `"system"` -> `ChatRole::System`
/// - `"tool"` -> `ChatRole::Tool`
/// - 其他 -> 跳过（过滤掉）
///
/// # 注意
///
/// - `think_timing` 字段被初始化为空向量
/// - 无法识别的角色会被过滤掉
pub(crate) fn to_session_history(
    turns: &[ChatMessage],
) -> Vec<crate::session::ui_types::ChatMessage> {
    turns
        .iter()
        .filter_map(|turn| {
            let role = match turn.role.as_str() {
                "user" => crate::session::ui_types::ChatRole::User,
                "assistant" => crate::session::ui_types::ChatRole::Assistant,
                "system" => crate::session::ui_types::ChatRole::System,
                "tool" => crate::session::ui_types::ChatRole::Tool,
                _ => return None,
            };
            Some(crate::session::ui_types::ChatMessage {
                role,
                content: turn.content.clone(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "session_tests.rs"]
pub(crate) mod session_tests;
