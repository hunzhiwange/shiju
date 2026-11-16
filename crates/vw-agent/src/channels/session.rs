//! 通道会话映射与 processor 集成；映射绑定完整消息作用域。
use super::*;

pub(crate) use channel_support::session::sender_session_store;

pub(crate) async fn clear_sender_session_id(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> Result<()> {
    let snapshot = resolve_channel_message_snapshot(ctx, msg).await?;
    channel_support::session::clear_sender_session_scope(&snapshot.scope);
    Ok(())
}

/// 会话创建失败返回错误，不缓存伪造的会话 ID。
pub(crate) async fn resolve_or_create_sender_session_id(snapshot: &ChannelMessageSnapshot, msg: &traits::ChannelMessage, route: Option<ChannelRouteSelection>) -> Result<String> {
    crate::session::ui_store::with_tenant_scope(snapshot.scope.chat.tenant.clone(), channel_support::session::resolve_or_create_sender_session_id(&ChannelSessionHost, snapshot, msg, route)).await
}

pub(crate) use channel_support::session::{ChannelSessionOwnership, ownership_scope_key};

pub(crate) use channel_support::session::channel_session_ownership;

pub(crate) use channel_support::session::save_channel_session_route;

pub(crate) async fn owned_channel_sessions(snapshot: &ChannelMessageSnapshot) -> Result<Vec<crate::session::session::Info>> {
    channel_support::session::owned_channel_sessions(&ChannelSessionHost, snapshot).await
}

/// 会话与所有权均写入成功后才替换当前映射。
pub(crate) async fn create_channel_session(snapshot: &ChannelMessageSnapshot, route: Option<ChannelRouteSelection>) -> Result<String> {
    channel_support::session::create_channel_session(&ChannelSessionHost, snapshot, route).await
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
    Ok(collect_channel_response(req, delta_tx).await?.text)
}

pub(crate) use channel_support::session_events::ChannelProcessorResponse;

pub(crate) async fn collect_channel_response(
    mut req: crate::session::processor::Request,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
) -> Result<ChannelProcessorResponse> {

    let user_id = if req.persist_app_session_artifacts && req.assistant_message_id.is_none() {
        let user_id = crate::id::ascending(crate::id::Prefix::Message, None)?;
        req.assistant_message_id = Some(crate::id::ascending(crate::id::Prefix::Message, None)?);
        Some(user_id)
    } else {
        None
    };
    let response = ChannelProcessorResponse {
        user_id,
        assistant_id: req.assistant_message_id.clone(),
        ..Default::default()
    };
    if req.persist_app_session_artifacts && req.app_session_scope.is_none() {
        req.app_session_scope =
            crate::app::agent::session::processor::app_session_scope_from_root_async(
                req.root.as_deref(),
            )
            .await;
    }

    // 创建无界通道用于事件传递
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<
        session_port::PromptEvent,
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
                    match channel_prompt_event(ev) { Some(ev) => event_tx.send(ev).is_ok(), None => !event_tx.is_closed() }
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
    let processor_task = tokio::task::spawn_blocking(run_processor);

    channel_support::session_events::collect_channel_events(
        event_rx, response, delta_tx, processor_task,
        crate::session::runtime::is_successful_finish_reason,
    ).await
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
/// - `"developer"` -> `ChatRole::Developer`
/// - `"tool"` -> `ChatRole::Tool`
/// - 其他 -> 跳过（过滤掉）
///
/// # 注意
///
/// - `think_timing` 字段被初始化为空向量
/// - 无法识别的角色会被过滤掉
pub(crate) use channel_support::session_events::to_session_history;

#[cfg(test)]
#[path = "session_tests.rs"]
pub(crate) mod session_tests;

struct ChannelSessionHost;
impl channel_support::session::ChannelSessions for ChannelSessionHost {
    fn get<'a>(&'a self, id: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output=Result<crate::session::session::Info, crate::session::session::Error>> + Send + 'a>> {
        Box::pin(crate::session::session::get_any(id))
    }
    #[cfg(not(target_arch="wasm32"))]
    fn reserve<'a>(&'a self, snapshot: &'a ChannelMessageSnapshot) -> std::pin::Pin<Box<dyn std::future::Future<Output=Result<crate::session::session::Info>> + Send + 'a>> {
        Box::pin(async move {
            let directory = snapshot.directory.to_string_lossy().to_string();
            let context = crate::project::instance::Context { directory: directory.clone(), worktree: snapshot.project.worktree.clone(), project: snapshot.project.clone() };
            crate::project::instance::provide_context(context, crate::session::session::reserve_next(crate::session::session::CreateInput { parent_id: None, title: None, directory, permission: None })).await.map_err(Into::into)
        })
    }
    #[cfg(not(target_arch="wasm32"))]
    fn discard<'a>(&'a self, snapshot: &'a ChannelMessageSnapshot, info: crate::session::session::Info) -> std::pin::Pin<Box<dyn std::future::Future<Output=Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let tenant = snapshot.scope.chat.tenant.clone();
            let project = info.project_id;
            let id = info.id;
            crate::global_database::run_isolated(move || crate::session::ui_store::discard_pending_session(&tenant, &project, &id)).await??;
            Ok(())
        })
    }
}

use channel_runtime::session_events::channel_prompt_event;
