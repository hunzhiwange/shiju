//! 会话提示词运行流程。
//!
//! 该模块负责把持久化会话记录、ACP 客户端、事件写入器和输出 formatter
//! 串联起来，执行一次会话提示词或一次性执行，并在失败路径中保存可恢复状态。

use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use agent_client_protocol::{McpServer, StopReason};
use parking_lot::Mutex;

use crate::perf_metrics::{format_perf_metric, measure_perf, start_perf_timer};
use crate::prompt_content::{PromptInput, prompt_to_display_text};
use crate::queue_lease_store::wait_ms;
use crate::session_events::{
    SessionEventAppendOptions, SessionEventWriter, SessionEventWriterOptions,
};
use crate::session_persistence::{absolute_path, iso_now, resolve_session_record};
use crate::session_runtime::connect_load::{
    ConnectAndLoadClient, ConnectAndLoadSessionOptions, connect_and_load_session,
};
use crate::session_runtime::lifecycle::apply_lifecycle_snapshot_to_record;
use crate::session_runtime::prompt_runner::PromptRunnerActiveSessionController;
use crate::session_runtime::prompt_runner::{
    ClientAvailableCallback, ClientClosedCallback, parse_agent_command,
};
use crate::session_runtime_helpers::with_timeout;
use crate::types::{
    AcpAgentConfig, AcpJsonRpcMessage, AcpMessageCallback, AcpSessionOptions, AuthPolicy,
    ClientOperationCallback, NonInteractivePermissionPolicy, OutputFormatter,
    OutputFormatterContext, PermissionMode, RunPromptResult, SessionResumePolicy,
    SessionSendResult, SessionUpdateCallback,
};
use crate::{
    AcpClient, PromptRequest, SessionStrategy,
    is_retryable_prompt_error as is_retryable_prompt_error_value,
};

use super::RunOnceOptions;
use super::SessionRuntimeError;
use super::state::{
    RuntimeState, build_runtime_state_callbacks, duplicate_acp_message, flush_pending_messages,
    non_empty_trimmed, replay_output_buffer, replay_output_messages, session_options_from_record,
};

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;

/// 提示词真正进入 agent turn 时触发的回调。
///
/// 回调用于让队列所有者在代理进入活跃 turn 后再应用挂起的控制请求，
/// 避免在客户端还未准备好时过早取消或修改会话状态。
pub(super) type OnPromptActiveCallback = Arc<dyn Fn() + Send + Sync>;

/// 运行持久化会话提示词所需的全部输入。
///
/// 该结构体把会话定位、权限、认证、输出和生命周期回调显式列出，
/// 以便调用者在队列执行和直接执行之间复用同一条运行路径。
pub(super) struct RunSessionPromptOptions<'a> {
    /// VibeWindow 持久化会话记录 ID。
    pub(super) session_record_id: String,
    /// 本次要发送给代理的提示词内容。
    pub(super) prompt: PromptInput,
    /// 会话恢复策略；为空时允许恢复、加载或新建。
    pub(super) resume_policy: Option<SessionResumePolicy>,
    /// 本次会话暴露给代理的 MCP 服务器列表。
    pub(super) mcp_servers: Option<Vec<McpServer>>,
    /// 客户端侧权限模式。
    pub(super) permission_mode: PermissionMode,
    /// 非交互权限策略。
    pub(super) non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    /// ACP 认证凭据。
    pub(super) auth_credentials: Option<HashMap<String, String>>,
    /// 认证缺失时的策略。
    pub(super) auth_policy: Option<AuthPolicy>,
    /// 输出 formatter，用于写出 ACP 消息和最终结果。
    pub(super) output_formatter: &'a mut dyn OutputFormatter,
    /// 原始 ACP 消息回调。
    pub(super) on_acp_message: Option<AcpMessageCallback>,
    /// 会话更新回调。
    pub(super) on_session_update: Option<SessionUpdateCallback>,
    /// 客户端侧操作回调。
    pub(super) on_client_operation: Option<ClientOperationCallback>,
    /// 单次连接或提示词操作的超时时间。
    pub(super) timeout_ms: Option<u64>,
    /// 是否抑制 SDK 控制台错误。
    pub(super) suppress_sdk_console_errors: bool,
    /// 是否输出性能诊断。
    pub(super) verbose: bool,
    /// 无副作用失败时允许重试的次数。
    pub(super) prompt_retries: Option<u64>,
    /// ACP 客户端可用时提供控制器的回调。
    pub(super) on_client_available: Option<ClientAvailableCallback>,
    /// ACP 客户端关闭后的回调。
    pub(super) on_client_closed: Option<ClientClosedCallback>,
    /// 提示词进入活动状态时的回调。
    pub(super) on_prompt_active: Option<OnPromptActiveCallback>,
}

/// 解析最终用于启动 ACP 代理的配置。
///
/// 如果调用方提供结构化配置则直接使用；否则从命令行字符串解析。该函数不启动
/// 进程，也不校验命令是否存在。
pub(super) fn resolve_agent_config(
    agent_command: &str,
    agent_config: Option<AcpAgentConfig>,
) -> AcpAgentConfig {
    agent_config.unwrap_or_else(|| parse_agent_command(agent_command))
}

#[allow(clippy::too_many_arguments)]
fn build_client(
    agent_command: &str,
    agent_config: Option<AcpAgentConfig>,
    mcp_servers: Option<Vec<McpServer>>,
    permission_mode: PermissionMode,
    non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    auth_credentials: Option<HashMap<String, String>>,
    auth_policy: Option<AuthPolicy>,
    suppress_sdk_console_errors: bool,
    verbose: bool,
    session_options: Option<AcpSessionOptions>,
    on_acp_message: Option<AcpMessageCallback>,
    on_acp_output_message: Option<AcpMessageCallback>,
    on_session_update: Option<SessionUpdateCallback>,
    on_client_operation: Option<ClientOperationCallback>,
) -> AcpClient {
    AcpClient::new(agent_command.to_string(), resolve_agent_config(agent_command, agent_config))
        .with_mcp_servers(mcp_servers.unwrap_or_default())
        .with_permission_mode(permission_mode)
        .with_non_interactive_permissions(non_interactive_permissions)
        .with_auth_credentials(auth_credentials.unwrap_or_default())
        .with_auth_policy(auth_policy.unwrap_or(AuthPolicy::Skip))
        .with_verbose(verbose || suppress_sdk_console_errors)
        .with_acp_message_callback(on_acp_message)
        .with_acp_output_message_callback(on_acp_output_message)
        .with_session_update_callback(on_session_update)
        .with_client_operation_callback(on_client_operation)
        .with_client_info("vibewindow-acp-client", env!("CARGO_PKG_VERSION"))
        .with_verbose(verbose)
        .with_session_options(session_options)
}

fn to_prompt_result(
    stop_reason: StopReason,
    session_id: String,
    client: &AcpClient,
) -> RunPromptResult {
    RunPromptResult { stop_reason, session_id, permission_stats: client.permission_stats() }
}

fn map_finish_reason(stop_reason: Option<&str>) -> StopReason {
    match stop_reason.unwrap_or("stop") {
        "length" => StopReason::MaxTokens,
        "max_turn_requests" => StopReason::MaxTurnRequests,
        "refusal" => StopReason::Refusal,
        "cancelled" => StopReason::Cancelled,
        _ => StopReason::EndTurn,
    }
}

fn emit_prompt_retry_notice(
    error: &SessionRuntimeError,
    delay_ms: u64,
    attempt: u64,
    max_retries: u64,
    suppress_sdk_console_errors: bool,
) {
    if suppress_sdk_console_errors {
        return;
    }

    eprintln!(
        "[vwacp] prompt failed ({}), retrying in {}ms (attempt {}/{})",
        error, delay_ms, attempt, max_retries
    );
}

fn prompt_request_for_session(
    session_id: String,
    cwd: &str,
    prompt: &PromptInput,
    resume_policy: Option<SessionResumePolicy>,
) -> PromptRequest {
    let strategy = if resume_policy == Some(SessionResumePolicy::SameSessionOnly) {
        SessionStrategy::ResumeOrLoad(session_id)
    } else {
        SessionStrategy::ResumeLoadOrNew(session_id)
    };

    PromptRequest {
        cwd: absolute_path(cwd),
        prompt: prompt_to_display_text(prompt),
        session_strategy: strategy,
    }
}

fn should_retry_prompt(error: &SessionRuntimeError) -> bool {
    if error.output_params().code != crate::OutputErrorCode::Runtime {
        return false;
    }

    if let Some(source) = StdError::source(error) {
        let message = source.to_string().to_ascii_lowercase();
        if message.contains("-32603") || message.contains("-32700") {
            return true;
        }
    }

    let value = serde_json::Value::String(error.to_string());
    is_retryable_prompt_error_value(&value)
}

/// 将会话选项中请求的模型应用到当前 ACP 会话。
///
/// 返回 `Ok(true)` 表示确实发送了模型切换请求，`Ok(false)` 表示没有有效模型。
/// 超时或代理拒绝会被包装为 [`SessionRuntimeError`]。
pub(super) async fn apply_requested_model(
    client: &AcpClient,
    session_id: &str,
    cwd: &str,
    requested_model: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<bool, SessionRuntimeError> {
    let Some(requested_model) = non_empty_trimmed(requested_model) else {
        return Ok(false);
    };

    with_timeout(
        client.set_session_model(session_id.to_string(), absolute_path(cwd), requested_model),
        timeout_ms,
    )
    .await
    .map_err(SessionRuntimeError::from_source)?
    .map_err(SessionRuntimeError::from_source)?;

    Ok(true)
}

/// 在持久化会话中执行一次提示词。
///
/// 函数会加载会话记录、创建事件写入器、构造 ACP 客户端、执行提示词并回写记录。
/// 成功时返回更新后的会话发送结果；失败时尽量刷新已收到输出并保存检查点，
/// 以便后续恢复和诊断。
pub(super) async fn run_session_prompt(
    options: RunSessionPromptOptions<'_>,
) -> Result<SessionSendResult, SessionRuntimeError> {
    let stop_total_timer = start_perf_timer("runtime.prompt.total");
    let mut record = measure_perf("session.resolve_prompt_record", || async {
        resolve_session_record(&options.session_record_id).await
    })
    .await
    .map_err(SessionRuntimeError::from_source)?;

    options
        .output_formatter
        .set_context(OutputFormatterContext { session_id: record.vwacp_record_id.clone() });

    let state = Arc::new(RuntimeState::new(&record, &options.prompt));
    let event_writer_record = record.clone();
    let mut event_writer = measure_perf("session.events.open", || async {
        SessionEventWriter::open(event_writer_record, SessionEventWriterOptions::default()).await
    })
    .await
    .map_err(SessionRuntimeError::from_source)?;
    let (on_acp_message, on_acp_output_message, on_session_update, on_client_operation) =
        build_runtime_state_callbacks(
            state.clone(),
            options.on_acp_message.clone(),
            options.on_session_update.clone(),
            options.on_client_operation.clone(),
        );

    let client = Arc::new(build_client(
        &record.agent_command,
        record.agent_config.clone(),
        options.mcp_servers.clone(),
        options.permission_mode,
        options.non_interactive_permissions,
        options.auth_credentials.clone(),
        options.auth_policy,
        options.suppress_sdk_console_errors,
        options.verbose,
        session_options_from_record(&record),
        on_acp_message,
        on_acp_output_message,
        on_session_update,
        on_client_operation,
    ));
    let active_session_id = Arc::new(Mutex::new(record.acp_session_id.clone()));
    let controller = Arc::new(PromptRunnerActiveSessionController::new(
        client.clone(),
        absolute_path(&record.cwd),
        active_session_id.clone(),
    ));
    let client_notified = options.on_client_available.is_some();
    let connect_started_at = Instant::now();
    let connect_result = connect_and_load_session(ConnectAndLoadSessionOptions {
        client: client.clone(),
        record: &mut record,
        resume_policy: options.resume_policy,
        timeout_ms: options.timeout_ms,
        verbose: options.verbose,
        active_controller: controller,
        on_client_available: options.on_client_available.clone(),
        on_connected_record: None,
        on_session_id_resolved: Some(Arc::new(move |id| {
            *active_session_id.lock() = id.to_string()
        })),
    })
    .await;
    let connect_result = match connect_result {
        Ok(result) => result,
        Err(error) => {
            if let Some(callback) = options.on_client_closed.as_ref() {
                callback();
            }
            client.close().await.ok();
            event_writer.close(SessionEventAppendOptions { checkpoint: true }).await.ok();
            return Err(SessionRuntimeError::from_source(error));
        }
    };
    state.sync_session_state(&record);

    record.last_prompt_at = Some(iso_now());
    state.set_buffering_connect_output(false);
    let connect_output_messages = state.take_connect_output_messages();
    replay_output_messages(options.output_formatter, connect_output_messages);

    if options.verbose {
        eprintln!(
            "[vwacp] {}",
            format_perf_metric(
                "prompt.connect_and_load",
                connect_started_at.elapsed().as_secs_f64() * 1_000.0,
            )
        );
    }

    flush_pending_messages(&mut event_writer, &state, false).await?;

    let mut prompt_result = None;
    state.mark_prompt_turn_active(true);
    for attempt in 0..=options.prompt_retries.unwrap_or(0) {
        let prompt_request = prompt_request_for_session(
            connect_result.session_id.clone(),
            &record.cwd,
            &options.prompt,
            options.resume_policy,
        );
        let mut active_notified = false;
        let mut on_event = |_event| {
            if !active_notified {
                if let Some(callback) = options.on_prompt_active.as_ref() {
                    callback();
                }
                active_notified = true;
            }
            replay_output_buffer(options.output_formatter, state.prompt_output_messages());
        };

        match measure_perf("runtime.prompt.agent_turn", || async {
            with_timeout(client.run_prompt(prompt_request, &mut on_event), options.timeout_ms).await
        })
        .await
        {
            Ok(Ok(result)) => {
                prompt_result = Some(result);
                break;
            }
            Ok(Err(error)) => {
                let error = SessionRuntimeError::from_source(error);
                if attempt < options.prompt_retries.unwrap_or(0)
                    && !state.prompt_turn_had_side_effects()
                    && should_retry_prompt(&error)
                {
                    let delay_ms = (1_000u64.saturating_mul(2u64.pow(attempt as u32))).min(10_000);
                    emit_prompt_retry_notice(
                        &error,
                        delay_ms,
                        attempt + 1,
                        options.prompt_retries.unwrap_or(0),
                        options.suppress_sdk_console_errors,
                    );
                    wait_ms(delay_ms).await;
                    if !state.prompt_turn_had_side_effects() {
                        continue;
                    }
                }
                state.mark_prompt_turn_active(false);
                flush_pending_messages(&mut event_writer, &state, false).await.ok();
                replay_output_messages(
                    options.output_formatter,
                    state.take_prompt_output_messages(),
                );
                options.output_formatter.flush();
                state.apply_to_record(&mut record);
                record.last_used_at = iso_now();
                let _ = event_writer.close(SessionEventAppendOptions { checkpoint: true }).await;
                if client_notified && let Some(callback) = options.on_client_closed {
                    callback();
                }
                return Err(error.with_output_already_emitted(state.saw_acp_message()));
            }
            Err(error) => {
                let error = SessionRuntimeError::from_source(error);
                state.mark_prompt_turn_active(false);
                flush_pending_messages(&mut event_writer, &state, false).await.ok();
                replay_output_messages(
                    options.output_formatter,
                    state.take_prompt_output_messages(),
                );
                options.output_formatter.flush();
                state.apply_to_record(&mut record);
                record.last_used_at = iso_now();
                let _ = event_writer.close(SessionEventAppendOptions { checkpoint: true }).await;
                if client_notified && let Some(callback) = options.on_client_closed {
                    callback();
                }
                return Err(SessionRuntimeError::from_source(error)
                    .with_output_already_emitted(state.saw_acp_message()));
            }
        }
    }
    state.mark_prompt_turn_active(false);

    flush_pending_messages(&mut event_writer, &state, false).await?;
    replay_output_messages(options.output_formatter, state.take_prompt_output_messages());
    options.output_formatter.flush();

    record.last_used_at = iso_now();
    record.closed = Some(false);
    record.closed_at = None;
    state.apply_to_record(&mut record);
    apply_lifecycle_snapshot_to_record(
        &mut record,
        &<AcpClient as ConnectAndLoadClient>::get_agent_lifecycle_snapshot(client.as_ref()),
    );
    event_writer
        .close(SessionEventAppendOptions { checkpoint: true })
        .await
        .map_err(SessionRuntimeError::from_source)?;

    if client_notified && let Some(callback) = options.on_client_closed {
        callback();
    }

    let elapsed_ms = stop_total_timer();
    if options.verbose {
        eprintln!("[vwacp] {}", format_perf_metric("prompt.total", elapsed_ms));
    }
    let prompt_result = prompt_result.expect("prompt result is set on success");
    Ok(SessionSendResult {
        stop_reason: map_finish_reason(prompt_result.finish_reason.as_deref()),
        permission_stats: client.permission_stats(),
        session_id: record.vwacp_record_id.clone(),
        record,
        resumed: connect_result.resumed,
        load_error: connect_result.load_error,
    })
}

/// 使用临时 ACP 会话执行一次提示词。
///
/// 该路径不会依赖持久化会话记录，适合 CLI 的一次性执行模式。成功返回 stop
/// reason、session id 和权限统计；失败时会先回放已缓存的 ACP 输出，再关闭客户端。
pub async fn run_once(options: RunOnceOptions<'_>) -> Result<RunPromptResult, SessionRuntimeError> {
    let on_acp_message = options.on_acp_message.clone();
    let on_acp_message_for_output = on_acp_message.clone();
    let on_session_update = options.on_session_update.clone();
    let on_client_operation = options.on_client_operation.clone();
    let output_formatter = &mut *options.output_formatter;
    let prompt_output_messages = Arc::new(Mutex::new(Vec::<AcpJsonRpcMessage>::new()));
    let saw_acp_message = Arc::new(AtomicBool::new(false));
    let prompt_turn_had_side_effects = Arc::new(AtomicBool::new(false));
    let client = Arc::new(build_client(
        &options.agent_command,
        options.agent_config.clone(),
        options.mcp_servers.clone(),
        options.permission_mode,
        options.non_interactive_permissions,
        options.auth_credentials.clone(),
        options.auth_policy,
        options.suppress_sdk_console_errors,
        options.verbose,
        options.session_options.clone(),
        Some(Arc::new({
            let saw_acp_message = saw_acp_message.clone();
            move |direction, message| {
                saw_acp_message.store(true, Ordering::SeqCst);
                if let Some(callback) = on_acp_message.as_ref() {
                    callback(direction, message);
                }
            }
        })),
        Some(Arc::new({
            let prompt_output_messages = prompt_output_messages.clone();
            let on_acp_message = on_acp_message_for_output.clone();
            move |direction, message| {
                if let Some(duplicate) = duplicate_acp_message(&message) {
                    prompt_output_messages.lock().push(duplicate);
                }
                if let Some(callback) = on_acp_message.as_ref() {
                    callback(direction, message);
                }
            }
        })),
        Some(Arc::new({
            let prompt_turn_had_side_effects = prompt_turn_had_side_effects.clone();
            move |notification| {
                prompt_turn_had_side_effects.store(true, Ordering::SeqCst);
                if let Some(callback) = on_session_update.as_ref() {
                    callback(notification);
                }
            }
        })),
        Some(Arc::new({
            let prompt_turn_had_side_effects = prompt_turn_had_side_effects.clone();
            move |operation| {
                prompt_turn_had_side_effects.store(true, Ordering::SeqCst);
                if let Some(callback) = on_client_operation.as_ref() {
                    callback(operation);
                }
            }
        })),
    ));

    let created_session = measure_perf("runtime.exec.create_session", || async {
        with_timeout(client.create_session(absolute_path(&options.cwd)), options.timeout_ms).await
    })
    .await
    .map_err(SessionRuntimeError::from_source)?
    .map_err(SessionRuntimeError::from_source)?;
    let session_id = created_session.session_id;
    let _ = apply_requested_model(
        client.as_ref(),
        &session_id,
        &options.cwd,
        options.session_options.as_ref().and_then(|value| value.model.as_deref()),
        options.timeout_ms,
    )
    .await?;

    output_formatter.set_context(OutputFormatterContext { session_id: session_id.clone() });

    let mut prompt_result = None;
    for attempt in 0..=options.prompt_retries.unwrap_or(0) {
        let prompt_request =
            prompt_request_for_session(session_id.clone(), &options.cwd, &options.prompt, None);
        let mut on_event = |_event| {
            replay_output_buffer(output_formatter, &prompt_output_messages);
        };
        match measure_perf("runtime.exec.prompt", || async {
            with_timeout(client.run_prompt(prompt_request, &mut on_event), options.timeout_ms).await
        })
        .await
        {
            Ok(Ok(result)) => {
                prompt_result = Some(result);
                break;
            }
            Ok(Err(error)) => {
                let error = SessionRuntimeError::from_source(error);
                if attempt < options.prompt_retries.unwrap_or(0)
                    && !prompt_turn_had_side_effects.load(Ordering::SeqCst)
                    && should_retry_prompt(&error)
                {
                    let delay_ms = (1_000u64.saturating_mul(2u64.pow(attempt as u32))).min(10_000);
                    emit_prompt_retry_notice(
                        &error,
                        delay_ms,
                        attempt + 1,
                        options.prompt_retries.unwrap_or(0),
                        options.suppress_sdk_console_errors,
                    );
                    wait_ms(delay_ms).await;
                    if !prompt_turn_had_side_effects.load(Ordering::SeqCst) {
                        continue;
                    }
                }
                replay_output_buffer(output_formatter, &prompt_output_messages);
                let _ = client.close().await;
                return Err(
                    error.with_output_already_emitted(saw_acp_message.load(Ordering::SeqCst))
                );
            }
            Err(error) => {
                replay_output_buffer(output_formatter, &prompt_output_messages);
                let _ = client.close().await;
                return Err(SessionRuntimeError::from_source(error)
                    .with_output_already_emitted(saw_acp_message.load(Ordering::SeqCst)));
            }
        }
    }

    replay_output_buffer(output_formatter, &prompt_output_messages);

    let prompt_result = prompt_result.expect("prompt result is set on success");
    let result = to_prompt_result(
        map_finish_reason(prompt_result.finish_reason.as_deref()),
        session_id,
        client.as_ref(),
    );
    let _ = client.close().await;
    Ok(result)
}
