//! 会话队列所有者运行逻辑。
//!
//! 本模块负责把提示发送给已有队列所有者、在需要时启动所有者进程，
//! 并在队列所有者内部串行处理会话任务。队列控制接口只暴露取消和会话
//! 配置更新等明确能力，不支持的控制路径会返回显式错误。

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::QUEUE_CONNECT_RETRY_MS;
use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::perf_metrics::set_perf_gauge;
use crate::perf_metrics_capture::checkpoint_perf_metrics_capture;
use crate::prompt_content::prompt_to_display_text;
use crate::queue_ipc::{
    SubmitToQueueOwnerOptions, try_cancel_on_running_owner, try_submit_to_running_owner,
};
use crate::queue_ipc_health::probe_queue_owner_health;
use crate::queue_ipc_server::{
    QueueOwnerControlHandlers, QueueOwnerSocketLease, QueueTask, SessionQueueOwner,
    SessionQueueOwnerOptions,
};
use crate::queue_lease_store::{
    refresh_queue_owner_lease, release_queue_owner_lease, try_acquire_default_queue_owner_lease,
    wait_ms,
};
use crate::queue_messages::QueueOwnerMessage;
use crate::queue_owner_turn_controller::QueueOwnerTurnController;
#[cfg(test)]
use crate::queue_owner_turn_controller::{QueueControlFuture, QueueOwnerTurnControllerOptions};
use crate::session_runtime::prompt_runner::{ClientAvailableCallback, ClientClosedCallback};
use crate::session_runtime::queue_owner_process::{
    QueueOwnerRuntimeOptions, QueueOwnerRuntimeSendOptions, queue_owner_runtime_options_from_send,
    spawn_queue_owner_process,
};
use crate::session_runtime_helpers::TimeoutError;
use crate::types::{OutputErrorOrigin, OutputFormatter, SessionSendOutcome, SessionSendResult};

use super::prompt::{OnPromptActiveCallback, RunSessionPromptOptions, run_session_prompt};
use super::state::{DiscardOutputFormatter, QueueTaskOutputFormatter};
use super::{
    DEFAULT_QUEUE_OWNER_TTL_MS, QUEUE_OWNER_HEARTBEAT_INTERVAL_MS,
    QUEUE_OWNER_STARTUP_MAX_ATTEMPTS, SessionCancelOptions, SessionCancelResult,
    SessionRuntimeError, SessionSendOptions,
};

#[cfg(test)]
#[path = "queue_owner_tests.rs"]
mod queue_owner_tests;

struct QueueOwnerControlBridge {
    controller: Arc<tokio::sync::Mutex<QueueOwnerTurnController>>,
}

#[async_trait::async_trait]
impl QueueOwnerControlHandlers for QueueOwnerControlBridge {
    async fn cancel_prompt(&self) -> Result<bool, QueueConnectionError> {
        let mut controller = self.controller.lock().await;
        let accepted = controller.request_cancel().await?;
        if !accepted {
            return Ok(false);
        }
        if !controller.has_pending_cancel() {
            return Ok(true);
        }
        controller.apply_pending_cancel().await
    }

    async fn set_session_mode(
        &self,
        mode_id: String,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        self.controller.lock().await.set_session_mode(mode_id, timeout_ms).await
    }

    async fn set_session_model(
        &self,
        model_id: String,
        timeout_ms: Option<u64>,
    ) -> Result<(), QueueConnectionError> {
        self.controller.lock().await.set_session_model(model_id, timeout_ms).await
    }

    async fn set_session_config_option(
        &self,
        config_id: String,
        value: String,
        timeout_ms: Option<u64>,
    ) -> Result<agent_client_protocol::SetSessionConfigOptionResponse, QueueConnectionError> {
        self.controller.lock().await.set_session_config_option(config_id, value, timeout_ms).await
    }
}

pub(super) fn timeout_queue_connection_error(error: TimeoutError) -> QueueConnectionError {
    QueueConnectionError::new(
        error.to_string(),
        AcpxErrorOptions {
            source: Some(Box::new(error)),
            detail_code: Some("TIMEOUT".to_string()),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(true),
            ..AcpxErrorOptions::default()
        },
    )
}

fn normalize_queue_owner_ttl_inner(ttl_ms: Option<u64>) -> u64 {
    ttl_ms.unwrap_or(DEFAULT_QUEUE_OWNER_TTL_MS)
}

/// 规范化队列所有者的存活时间。
///
/// `ttl_ms` 为空时返回默认 TTL；传入 `0` 时保留为无限等待语义。
/// 返回值直接用于所有者任务轮询和日志展示。
pub fn normalize_queue_owner_ttl_ms(ttl_ms: Option<u64>) -> u64 {
    normalize_queue_owner_ttl_inner(ttl_ms)
}

/// 将提示发送到会话队列，必要时启动队列所有者。
///
/// 优先复用正在运行的所有者；如果不存在则派生所有者进程并重试提交。
/// 当调用方要求等待完成且所有者启动失败但没有有效租约时，会安全回退到
/// 直接执行路径。提交、启动或直接执行失败会返回 [`SessionRuntimeError`]。
pub async fn send_session(
    options: SessionSendOptions<'_>,
) -> Result<SessionSendOutcome, SessionRuntimeError> {
    let wait_for_completion = options.wait_for_completion;
    let message = prompt_to_display_text(&options.prompt);
    let queued_to_owner = try_submit_to_running_owner(&mut SubmitToQueueOwnerOptions {
        session_id: &options.session_id,
        message,
        prompt: Some(options.prompt.clone()),
        permission_mode: options.permission_mode,
        resume_policy: options.resume_policy,
        non_interactive_permissions: options.non_interactive_permissions,
        output_formatter: options.output_formatter,
        error_emission_policy: options.error_emission_policy.clone(),
        timeout_ms: options.timeout_ms,
        suppress_sdk_console_errors: Some(options.suppress_sdk_console_errors),
        wait_for_completion,
        verbose: options.verbose,
    })
    .await
    .map_err(SessionRuntimeError::from_source)?;
    if let Some(outcome) = queued_to_owner {
        return Ok(outcome);
    }

    spawn_queue_owner_process(
        &queue_owner_runtime_options_from_send(&QueueOwnerRuntimeSendOptions {
            session_id: options.session_id.clone(),
            mcp_servers: options.mcp_servers.clone(),
            permission_mode: options.permission_mode,
            non_interactive_permissions: options.non_interactive_permissions,
            auth_credentials: options.auth_credentials.clone(),
            auth_policy: options.auth_policy,
            suppress_sdk_console_errors: Some(options.suppress_sdk_console_errors),
            verbose: Some(options.verbose),
            ttl_ms: options.ttl_ms,
            max_queue_depth: options.max_queue_depth,
            prompt_retries: options.prompt_retries,
        }),
        None,
    )
    .map_err(SessionRuntimeError::from_source)?;

    for _ in 0..QUEUE_OWNER_STARTUP_MAX_ATTEMPTS {
        let queued = try_submit_to_running_owner(&mut SubmitToQueueOwnerOptions {
            session_id: &options.session_id,
            message: prompt_to_display_text(&options.prompt),
            prompt: Some(options.prompt.clone()),
            permission_mode: options.permission_mode,
            resume_policy: options.resume_policy,
            non_interactive_permissions: options.non_interactive_permissions,
            output_formatter: options.output_formatter,
            error_emission_policy: options.error_emission_policy.clone(),
            timeout_ms: options.timeout_ms,
            suppress_sdk_console_errors: Some(options.suppress_sdk_console_errors),
            wait_for_completion,
            verbose: options.verbose,
        })
        .await
        .map_err(SessionRuntimeError::from_source)?;
        if let Some(outcome) = queued {
            return Ok(outcome);
        }
        wait_ms(QUEUE_CONNECT_RETRY_MS).await;
    }

    let health = probe_queue_owner_health(&options.session_id).await;
    if wait_for_completion && !health.has_lease {
        if options.verbose {
            eprintln!(
                "[vwacp] queue owner unavailable for session {}; falling back to direct prompt execution",
                options.session_id
            );
        }
        return send_session_direct(options)
            .await
            .map(|result| SessionSendOutcome::SessionSendResult(Box::new(result)));
    }

    Err(SessionRuntimeError::from_source(io::Error::other(format!(
        "Session queue owner failed to start for session {}",
        options.session_id
    ))))
}

/// 绕过队列所有者，直接在当前进程执行提示。
///
/// 该路径复用 [`run_session_prompt`]，用于显式直接运行或队列启动失败后的
/// 安全回退。返回会话发送结果；客户端和运行时错误会被保留为
/// [`SessionRuntimeError`]。
pub async fn send_session_direct(
    options: SessionSendOptions<'_>,
) -> Result<SessionSendResult, SessionRuntimeError> {
    run_session_prompt(RunSessionPromptOptions {
        session_record_id: options.session_id,
        prompt: options.prompt,
        resume_policy: options.resume_policy,
        mcp_servers: options.mcp_servers,
        permission_mode: options.permission_mode,
        non_interactive_permissions: options.non_interactive_permissions,
        auth_credentials: options.auth_credentials,
        auth_policy: options.auth_policy,
        output_formatter: options.output_formatter,
        on_acp_message: options.on_acp_message,
        on_session_update: options.on_session_update,
        on_client_operation: options.on_client_operation,
        timeout_ms: options.timeout_ms,
        suppress_sdk_console_errors: options.suppress_sdk_console_errors,
        verbose: options.verbose,
        prompt_retries: options.prompt_retries,
        on_client_available: None,
        on_client_closed: None,
        on_prompt_active: None,
    })
    .await
}

/// 请求取消正在运行的会话提示。
///
/// 函数只与当前会话的运行中队列所有者通信；没有可取消任务时返回
/// `cancelled = false`。IPC 或队列错误会转换为 [`SessionRuntimeError`]。
pub async fn cancel_session_prompt(
    options: SessionCancelOptions,
) -> Result<SessionCancelResult, SessionRuntimeError> {
    let cancelled = try_cancel_on_running_owner(&options.session_id, options.verbose)
        .await
        .map_err(SessionRuntimeError::from_source)?;
    Ok(SessionCancelResult { session_id: options.session_id, cancelled: cancelled == Some(true) })
}

/// 运行单个会话的队列所有者循环。
///
/// 启动时先获取默认队列租约，随后监听提交的任务并逐个执行。
/// 心跳会刷新租约和队列深度，关闭时释放租约并停止 socket 服务。
/// 无法获取租约表示已有所有者接管，函数会直接返回 `Ok(())`。
pub async fn run_session_queue_owner(
    options: QueueOwnerRuntimeOptions,
) -> Result<(), SessionRuntimeError> {
    let Some(lease) = try_acquire_default_queue_owner_lease(&options.session_id)
        .await
        .map_err(SessionRuntimeError::from_source)?
    else {
        return Ok(());
    };

    let turn_controller = Arc::new(tokio::sync::Mutex::new(QueueOwnerTurnController::new(
        super::queue_owner_controls::turn_controller_options(&options),
    )));

    let control_handlers: Arc<dyn QueueOwnerControlHandlers> =
        Arc::new(QueueOwnerControlBridge { controller: turn_controller.clone() });
    let queue_depth = Arc::new(AtomicU64::new(0));
    let queue_owner = match SessionQueueOwner::start(
        QueueOwnerSocketLease {
            socket_path: lease.socket_path.clone(),
            owner_generation: Some(lease.owner_generation),
        },
        control_handlers,
        SessionQueueOwnerOptions {
            max_queue_depth: options.max_queue_depth,
            on_queue_depth_changed: Some(Arc::new({
                let lease = lease.clone();
                let queue_depth = queue_depth.clone();
                move |depth| {
                    queue_depth.store(depth as u64, Ordering::SeqCst);
                    set_perf_gauge("queue.owner.depth", depth as f64);
                    let lease = lease.clone();
                    tokio::spawn(async move {
                        let _ = refresh_queue_owner_lease(&lease, depth as u64).await;
                    });
                }
            })),
        },
    )
    .await
    {
        Ok(queue_owner) => queue_owner,
        Err(error) => {
            let _ = release_queue_owner_lease(&lease).await;
            return Err(SessionRuntimeError::from_source(error));
        }
    };

    if options.verbose.unwrap_or(false) {
        eprintln!(
            "[vwacp] queue owner ready for session {} (ttlMs={}, maxQueueDepth={})",
            options.session_id,
            normalize_queue_owner_ttl_ms(options.ttl_ms),
            options.max_queue_depth.unwrap_or(16).max(1),
        );
    }

    let heartbeat_lease = lease.clone();
    let heartbeat_depth = queue_depth.clone();
    let heartbeat_handle = tokio::spawn(async move {
        loop {
            wait_ms(QUEUE_OWNER_HEARTBEAT_INTERVAL_MS).await;
            let _ =
                refresh_queue_owner_lease(&heartbeat_lease, heartbeat_depth.load(Ordering::SeqCst))
                    .await;
        }
    });

    let ttl_ms = normalize_queue_owner_ttl_ms(options.ttl_ms);
    let task_poll_timeout_ms = (ttl_ms != 0).then_some(ttl_ms);
    let initial_task_poll_timeout_ms = task_poll_timeout_ms.map(|timeout_ms| timeout_ms.max(1_000));
    let mut queue_owner = Some(queue_owner);
    let mut is_first_task = true;

    let result = async {
        while let Some(owner) = queue_owner.as_ref() {
            let poll_timeout_ms =
                if is_first_task { initial_task_poll_timeout_ms } else { task_poll_timeout_ms };
            let Some(task) = owner.next_task(poll_timeout_ms).await else {
                break;
            };
            is_first_task = false;

            {
                let mut controller = turn_controller.lock().await;
                controller.begin_turn();
            }

            let task_result = run_queued_task(
                &options.session_id,
                task.clone(),
                &options,
                turn_controller.clone(),
            )
            .await;
            {
                let mut controller = turn_controller.lock().await;
                controller.end_turn();
            }
            checkpoint_perf_metrics_capture();
            task_result?;
        }
        Ok::<(), SessionRuntimeError>(())
    }
    .await;

    heartbeat_handle.abort();
    if let Some(owner) = queue_owner.take() {
        {
            let mut controller = turn_controller.lock().await;
            controller.begin_closing();
        }
        let _ = owner.close().await;
    }
    let _ = release_queue_owner_lease(&lease).await;

    if options.verbose.unwrap_or(false) {
        eprintln!("[vwacp] queue owner stopped for session {}", options.session_id);
    }

    result
}

async fn run_queued_task(
    session_record_id: &str,
    task: QueueTask,
    options: &QueueOwnerRuntimeOptions,
    turn_controller: Arc<tokio::sync::Mutex<QueueOwnerTurnController>>,
) -> Result<(), SessionRuntimeError> {
    let mut formatter = QueueTaskOutputFormatter::new(task.request_id.clone(), task.clone());
    let mut discard = DiscardOutputFormatter;
    let output_formatter: &mut dyn OutputFormatter =
        if task.wait_for_completion { &mut formatter } else { &mut discard };

    let verbose = options.verbose.unwrap_or(false);
    let on_client_available: ClientAvailableCallback = Arc::new({
        let turn_controller = turn_controller.clone();
        move |active| {
            let turn_controller = turn_controller.clone();
            tokio::spawn(async move {
                turn_controller.lock().await.set_active_controller(active);
            });
        }
    });
    let on_client_closed: ClientClosedCallback = Arc::new({
        let turn_controller = turn_controller.clone();
        move || {
            let turn_controller = turn_controller.clone();
            tokio::spawn(async move {
                turn_controller.lock().await.clear_active_controller();
            });
        }
    });
    let on_prompt_active: OnPromptActiveCallback = Arc::new({
        let turn_controller = turn_controller.clone();
        move || {
            let turn_controller = turn_controller.clone();
            tokio::spawn(async move {
                let mut guard = turn_controller.lock().await;
                guard.mark_prompt_active();
                let _ = guard.apply_pending_cancel().await;
            });
        }
    });

    let result = run_session_prompt(RunSessionPromptOptions {
        session_record_id: session_record_id.to_string(),
        prompt: task.prompt.clone(),
        resume_policy: task.resume_policy,
        mcp_servers: options.mcp_servers.clone(),
        permission_mode: task.permission_mode,
        non_interactive_permissions: task
            .non_interactive_permissions
            .or(options.non_interactive_permissions),
        auth_credentials: options.auth_credentials.clone(),
        auth_policy: options.auth_policy,
        output_formatter,
        on_acp_message: None,
        on_session_update: None,
        on_client_operation: None,
        timeout_ms: task.timeout_ms,
        suppress_sdk_console_errors: task
            .suppress_sdk_console_errors
            .unwrap_or(options.suppress_sdk_console_errors.unwrap_or(false)),
        verbose,
        prompt_retries: options.prompt_retries,
        on_client_available: Some(on_client_available),
        on_client_closed: Some(on_client_closed),
        on_prompt_active: Some(on_prompt_active),
    })
    .await;

    formatter.finish().await;

    match result {
        Ok(result) => {
            if task.wait_for_completion {
                task.send(QueueOwnerMessage::Result {
                    request_id: task.request_id.clone(),
                    owner_generation: None,
                    result: Box::new(result),
                })
                .await;
            }
            task.close().await;
            Ok(())
        }
        Err(error) => {
            if task.wait_for_completion {
                let output = error.output_params();
                task.send(QueueOwnerMessage::Error {
                    request_id: task.request_id.clone(),
                    owner_generation: None,
                    code: output.code,
                    detail_code: output
                        .detail_code
                        .or_else(|| Some("QUEUE_RUNTIME_PROMPT_FAILED".to_string())),
                    origin: output.origin.unwrap_or(OutputErrorOrigin::Runtime),
                    message: output.message,
                    retryable: output.retryable,
                    acp: output.acp,
                    output_already_emitted: Some(
                        error.output_already_emitted() || formatter.has_messages(),
                    ),
                })
                .await;
            }
            task.close().await;
            if error.is_interrupted() {
                return Err(error);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
fn unsupported_queue_control_error() -> QueueConnectionError {
    QueueConnectionError::new(
        "Queue owner control fallback is unavailable",
        AcpxErrorOptions {
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(true),
            ..AcpxErrorOptions::default()
        },
    )
}

#[cfg(test)]
fn unsupported_queue_control_future() -> QueueControlFuture<()> {
    Box::pin(async { Err(unsupported_queue_control_error()) })
}

#[cfg(test)]
fn unsupported_queue_config_option_future()
-> QueueControlFuture<agent_client_protocol::SetSessionConfigOptionResponse> {
    Box::pin(async { Err(unsupported_queue_control_error()) })
}
