//! 消息执行模块
//!
//! 本模块负责在通道管理器中执行消息处理流程，将接收到的用户消息
//! 通过会话处理器发送给 LLM 进行处理，并返回执行结果。
//!
//! # 主要功能
//!
//! - 解析或创建发送者会话标识
//! - 构建会话处理器请求
//! - 管理执行超时和取消机制
//! - 返回结构化的 LLM 执行结果

use super::*;

fn channel_session_options(route: &ChannelRouteSelection) -> serde_json::Value {
    let mut options = serde_json::json!({ "full_access": true });
    if let Some(effort) = &route.reasoning_effort {
        options["reasoning_effort"] = serde_json::json!(effort);
    }
    options
}

fn channel_request_reliability(
    ctx: &ChannelRuntimeContext,
) -> crate::app::agent::config::ReliabilityConfig {
    ctx.reliability.as_ref().clone()
}

/// 执行消息处理流程
///
/// 该函数是通道管理器中处理用户消息的核心入口点，负责将消息
/// 提交给会话处理器进行 LLM 推理，并管理整个执行生命周期。
///
/// # 参数
///
/// * `ctx` - 通道运行时上下文，包含通道配置和状态信息
/// * `msg` - 待处理的通道消息，包含用户输入内容
/// * `route` - 通道路由选择，指定使用的模型和路由策略
/// * `session_history` - 会话历史记录，用于上下文连续性
/// * `delta_tx` - 可选的增量文本发送通道，用于流式输出
/// * `timeout_budget_secs` - 执行超时预算（秒）
/// * `cancellation_token` - 取消令牌，用于优雅中断执行
///
/// # 返回值
///
/// 返回 `LlmExecutionResult` 枚举，表示执行结果：
/// - `Completed` - 执行完成，包含会话处理器的处理结果
/// - `Cancelled` - 执行被取消
///
/// # 执行流程
///
/// 1. 解析或创建发送者的会话 ID
/// 2. 确定通道的项目目录作为请求根目录
/// 3. 使用 tokio::select! 同时监听取消信号和执行结果
/// 4. 在超时限制内执行会话处理器
/// 5. 返回执行结果
///
/// # 示例
///
/// ```ignore
/// let result = run_message_execution(
///     &ctx,
///     &message,
///     &route,
///     history,
///     Some(delta_channel),
///     300,
///     &cancel_token,
/// ).await;
/// ```
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_message_execution(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    snapshot: &ChannelMessageSnapshot,
    session_history: Vec<crate::session::ui_types::ChatMessage>,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    timeout_budget_secs: u64,
    cancellation_token: &CancellationToken,
) -> LlmExecutionResult {
    if cancellation_token.is_cancelled() {
        return LlmExecutionResult::Cancelled;
    }
    let session_id = match resolve_or_create_sender_session_id(snapshot, msg).await {
        Ok(id) => id,
        Err(err) => return LlmExecutionResult::Completed(Ok(Err(err))),
    };
    if let Err(err) = persist_current_route(ctx, route).await {
        return LlmExecutionResult::Completed(Ok(Err(err)));
    }
    let request_root_dir = &snapshot.directory;
    let (approval_prompt_tx, _approval_prompt_rx) = tokio::sync::mpsc::unbounded_channel();

    // 使用 select! 同时监听取消信号和执行结果
    // 优先响应取消信号，确保快速终止
    crate::session::ui_store::with_tenant_scope(snapshot.scope.chat.tenant.clone(), async {
        tokio::select! {
            // 如果收到取消信号，立即返回 Cancelled
            () = cancellation_token.cancelled() => LlmExecutionResult::Cancelled,

            // 在指定的超时时间内执行会话处理器
            result = tokio::time::timeout(
                Duration::from_secs(timeout_budget_secs),
                run_session_processor_for_channel(
                    // 构建会话处理器请求
                    crate::app::agent::session::processor::Request {
                        // 使用当前时间戳作为流标识
                        stream: crate::app::agent::session::session::now_ms(),
                        // 使用解析或创建的会话 ID
                        session: session_id,
                        // 用户消息内容
                        query: msg.content.clone(),
                        // 设置请求的工作目录
                        root: Some(request_root_dir.to_string_lossy().to_string()),
                        // 指定使用的模型
                        model: Some(channel_model_request_model(&route.provider, &route.model)),
                        reliability: channel_request_reliability(ctx),
                        options: channel_session_options(route),
                        approval: Some(ctx.approval_manager.clone()),
                        channel_name: Some(msg.channel.clone()),
                        non_cli_approval_context: Some(crate::agent::loop_::NonCliApprovalContext {
                            sender: msg.sender.clone(),
                            reply_target: msg.reply_target.clone(),
                            prompt_tx: approval_prompt_tx,
                        }),
                        assistant_message_id: None,
                        // 传入会话历史以保持上下文连续性
                        history: session_history,
                        // 持久化会话工件（如日志、中间结果等）
                        persist_app_session_artifacts: true,
                        app_session_scope: Some(snapshot.scope.project.clone()),
                        abort_token: None,
                        session_tools: crate::session::runtime::SessionToolset::default(),
                    },
                    // 传入增量文本通道，用于流式输出
                    delta_tx,
                ),
            ) => LlmExecutionResult::Completed(result),
        }
    })
    .await
}

#[cfg(test)]
#[path = "message_execution_tests.rs"]
mod message_execution_tests;
