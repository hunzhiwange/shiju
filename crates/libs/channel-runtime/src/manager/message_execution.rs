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

#[doc(hidden)]
pub fn channel_session_options(route: &ChannelRouteSelection, full_access: bool) -> serde_json::Value {
    let mut options = serde_json::json!({ "full_access": full_access });
    if let Some(effort) = &route.reasoning_effort {
        options["reasoning_effort"] = serde_json::json!(effort);
    }
    options
}

#[doc(hidden)]
pub fn channel_request_reliability(
    ctx: &ChannelRuntimeContext,
) -> agent_config::runtime_schema::ReliabilityConfig {
    ctx.reliability.as_ref().clone()
}

// The channel calls the processor directly, bypassing PromptCancelHandle.
// Reap explicitly here; dropping a response/wait future must not do this.
#[doc(hidden)]
pub async fn stop_channel_execution(
    host: &dyn crate::host::ChannelHost,
    execution: impl std::future::Future<Output = anyhow::Result<ChannelProcessorResponse>>,
    abort: &CancellationToken,
    snapshot: &ChannelMessageSnapshot,
    session: &str,
) -> anyhow::Result<()> {
    abort.cancel();
    let first_cleanup = host.reap_shell(snapshot, session);
    // A full delta channel must not make the deadline wait forever. Dropping
    // this receiver also stops event delivery from the blocking processor.
    let _ = tokio::time::timeout(Duration::from_secs(5), execution).await;
    // Catch a tool that crossed spawn concurrently with cancellation.
    host.reap_shell(snapshot, session)?;
    first_cleanup
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
pub async fn run_message_execution(
    host: &'static dyn crate::host::ChannelHost,
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    snapshot: &ChannelMessageSnapshot,
    session_history: Vec<session_port::ui_types::ChatMessage>,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    timeout_budget_secs: u64,
    cancellation_token: &CancellationToken,
) -> LlmExecutionResult {
    if cancellation_token.is_cancelled() {
        return LlmExecutionResult::Cancelled;
    }
    let session_id =
        match host.session_id(snapshot, msg, Some(route.clone())).await {
            Ok(id) => id,
            Err(err) => return LlmExecutionResult::Completed(Ok(Err(err))),
        };
    let options = match super::execution_binding::execution_options(
        host,
        snapshot,
        &session_id,
        route,
        channel_session_options(route, ctx.session_full_access),
    )
    .await
    {
        Ok(options) => options,
        Err(error) => return LlmExecutionResult::Completed(Ok(Err(error))),
    };
    if let Err(err) = persist_current_route(ctx, route).await {
        return LlmExecutionResult::Completed(Ok(Err(err)));
    }
    let request_root_dir = &snapshot.directory;
    let (approval_prompt_tx, _approval_prompt_rx) = tokio::sync::mpsc::unbounded_channel();

    let execution_started = tokio::time::Instant::now();
    let processor_abort = cancellation_token.child_token();
    let _abort_guard = processor_abort.clone().drop_guard();
    // 使用 select! 同时监听取消信号和执行结果
    // 优先响应取消信号，确保快速终止
    let scope_snapshot = snapshot.clone();
    let mut execution_result = LlmExecutionResult::Cancelled;
    host.in_tenant_scope(&scope_snapshot, Box::pin(async {
        let execution = host.collect_response(crate::host::ChannelExecutionInput {
            stream: host.now_ms(),
            session: session_id.clone(), query: msg.content.clone(), directory: request_root_dir.clone(),
            model: channel_model_request_model(&route.provider, &route.model),
            reliability: channel_request_reliability(ctx), options, approval: ctx.approval_manager.clone(),
            channel_name: msg.channel.clone(),
            non_cli_approval_context: tool_context::approval::NonCliApprovalContext { sender: msg.sender.clone(), reply_target: msg.reply_target.clone(), prompt_tx: approval_prompt_tx },
            history: session_history, project: snapshot.scope.project.clone(), abort: processor_abort.clone(),
        }, delta_tx);
        tokio::pin!(execution);
        execution_result = tokio::select! {
            biased;
            () = cancellation_token.cancelled() => {
                match stop_channel_execution(host, execution, &processor_abort, snapshot, &session_id).await {
                    Ok(()) => LlmExecutionResult::Cancelled,
                    Err(error) => LlmExecutionResult::Completed(Ok(Err(error.context("channel user cancellation: shell cleanup failed")))),
                }
            }
            () = tokio::time::sleep(Duration::from_secs(timeout_budget_secs)) => {
                let elapsed_ms = execution_started.elapsed().as_millis();
                let cleanup = stop_channel_execution(host, execution, &processor_abort, snapshot, &session_id).await;
                let status = match cleanup {
                    Ok(()) => "owned shell processes reaped; tail output retained for bash_wait".to_string(),
                    Err(error) => format!("shell cleanup failed: {error}"),
                };
                LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!(
                    "channel_message_timeout_budget exhausted: budget_secs={timeout_budget_secs}, elapsed_ms={elapsed_ms}; cancellation_reason=channel_deadline; {status}"
                ))))
            }
            result = &mut execution => {
                if cancellation_token.is_cancelled() {
                    match stop_channel_execution(host, std::future::ready(result), &processor_abort, snapshot, &session_id).await {
                        Ok(()) => LlmExecutionResult::Cancelled,
                        Err(error) => LlmExecutionResult::Completed(Ok(Err(error.context("channel user cancellation: shell cleanup failed")))),
                    }
                } else { LlmExecutionResult::Completed(Ok(result)) }
            }
        };
    }))
    .await;
    execution_result
}
