//! 客户端与队列所有者之间的 IPC 请求入口。
//!
//! 本模块提供前台 CLI 与后台队列所有者通信的统一入口函数，
//! 负责构造请求、建立连接、发送命令并读取返回消息。
//!
//! 典型调用包括提交 prompt、取消当前任务、切换会话模式、切换模型，
//! 以及更新运行中的配置选项。

use std::fmt::Display;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agent_client_protocol::SetSessionConfigOptionResponse;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::errors::{AcpxErrorOptions, QueueConnectionError};
use crate::perf_metrics::increment_perf_counter;
use crate::queue_ipc_health::probe_queue_owner_health;
use crate::queue_ipc_transport::connect_to_queue_owner;
use crate::queue_lease_store::{
    QueueOwnerRecord, read_default_queue_owner_record, terminate_default_queue_owner_for_session,
};
use crate::queue_messages::{QueueOwnerMessage, QueueRequest, parse_queue_owner_message};
use crate::types::{
    OutputErrorCode, OutputErrorEmissionPolicy, OutputErrorOrigin, OutputErrorParams,
    OutputFormatter, OutputFormatterContext, PermissionMode, SessionEnqueueResult,
    SessionResumePolicy, SessionSendOutcome,
};
use crate::{NonInteractivePermissionPolicy, PromptInput};

pub const MAX_MESSAGE_BUFFER_SIZE: usize = 10 * 1024 * 1024;

const STALE_OWNER_PROTOCOL_DETAIL_CODES: &[&str] =
    &["QUEUE_PROTOCOL_MALFORMED_MESSAGE", "QUEUE_PROTOCOL_UNEXPECTED_RESPONSE"];

static NEXT_QUEUE_REQUEST_ID: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(1));

pub fn next_queue_request_id() -> String {
    let counter = NEXT_QUEUE_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let timestamp_ms =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis();
    format!("queue-{timestamp_ms}-{counter}")
}

fn make_queue_connection_error(
    message: impl Into<String>,
    detail_code: impl Into<String>,
    retryable: bool,
) -> QueueConnectionError {
    QueueConnectionError::new(
        message,
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::Runtime),
            detail_code: Some(detail_code.into()),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(retryable),
            ..AcpxErrorOptions::default()
        },
    )
}

fn make_queue_connection_error_with_source(
    message: impl Into<String>,
    detail_code: impl Into<String>,
    retryable: bool,
    source: impl std::error::Error + Send + Sync + 'static,
) -> QueueConnectionError {
    QueueConnectionError::new(
        message,
        AcpxErrorOptions {
            source: Some(Box::new(source)),
            output_code: Some(OutputErrorCode::Runtime),
            detail_code: Some(detail_code.into()),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(retryable),
            ..AcpxErrorOptions::default()
        },
    )
}

fn is_stale_owner_protocol_error(error: &QueueConnectionError) -> bool {
    error
        .detail_code()
        .is_some_and(|detail_code| STALE_OWNER_PROTOCOL_DETAIL_CODES.contains(&detail_code))
}

async fn maybe_recover_stale_owner_after_protocol_mismatch(
    session_id: &str,
    _owner: &QueueOwnerRecord,
    error: &QueueConnectionError,
    verbose: bool,
) -> bool {
    if !is_stale_owner_protocol_error(error) {
        return false;
    }

    let _ = terminate_default_queue_owner_for_session(session_id).await;
    increment_perf_counter("queue.owner.stale_recovered", 1);

    if verbose {
        eprintln!(
            "[vwacp] dropped stale queue owner metadata after protocol mismatch for session {} ({})",
            session_id,
            error.detail_code().unwrap_or("QUEUE_PROTOCOL_ERROR")
        );
    }

    true
}

#[allow(clippy::result_large_err)]
fn assert_owner_generation(
    owner: &QueueOwnerRecord,
    message: QueueOwnerMessage,
) -> Result<QueueOwnerMessage, QueueConnectionError> {
    let message_owner_generation = match &message {
        QueueOwnerMessage::Accepted { owner_generation, .. }
        | QueueOwnerMessage::Event { owner_generation, .. }
        | QueueOwnerMessage::Result { owner_generation, .. }
        | QueueOwnerMessage::CancelResult { owner_generation, .. }
        | QueueOwnerMessage::SetModeResult { owner_generation, .. }
        | QueueOwnerMessage::SetModelResult { owner_generation, .. }
        | QueueOwnerMessage::SetConfigOptionResult { owner_generation, .. }
        | QueueOwnerMessage::Error { owner_generation, .. } => *owner_generation,
    };

    if let (Some(expected), Some(actual)) = (Some(owner.owner_generation), message_owner_generation)
        && actual != expected
    {
        return Err(make_queue_connection_error(
            "Queue owner returned mismatched generation",
            "QUEUE_OWNER_GENERATION_MISMATCH",
            true,
        ));
    }

    Ok(message)
}

#[allow(clippy::result_large_err)]
fn parse_queue_owner_response_line(
    owner: &QueueOwnerRecord,
    request_id: &str,
    line: &str,
) -> Result<QueueOwnerMessage, QueueConnectionError> {
    let parsed = serde_json::from_str::<Value>(line).map_err(|error| {
        make_queue_connection_error_with_source(
            "Queue owner sent invalid JSON payload",
            "QUEUE_PROTOCOL_INVALID_JSON",
            true,
            error,
        )
    })?;

    let parsed_message = parse_queue_owner_message(&parsed).ok_or_else(|| {
        make_queue_connection_error(
            "Queue owner sent malformed message",
            "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
            true,
        )
    })?;

    let message = assert_owner_generation(owner, parsed_message)?;
    let response_request_id = queue_owner_message_request_id(&message);
    if response_request_id != request_id {
        return Err(make_queue_connection_error(
            "Queue owner sent malformed message",
            "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
            true,
        ));
    }

    Ok(message)
}

fn queue_owner_message_request_id(message: &QueueOwnerMessage) -> &str {
    match message {
        QueueOwnerMessage::Accepted { request_id, .. }
        | QueueOwnerMessage::Event { request_id, .. }
        | QueueOwnerMessage::Result { request_id, .. }
        | QueueOwnerMessage::CancelResult { request_id, .. }
        | QueueOwnerMessage::SetModeResult { request_id, .. }
        | QueueOwnerMessage::SetModelResult { request_id, .. }
        | QueueOwnerMessage::SetConfigOptionResult { request_id, .. }
        | QueueOwnerMessage::Error { request_id, .. } => request_id,
    }
}

async fn run_queue_owner_request<TResult, FAccepted, FMessage, FClose>(
    owner: &QueueOwnerRecord,
    request: &QueueRequest,
    mut on_accepted: FAccepted,
    mut on_message: FMessage,
    mut on_close: FClose,
) -> Result<Option<TResult>, QueueConnectionError>
where
    FAccepted: FnMut() -> Result<Option<TResult>, QueueConnectionError>,
    FMessage: FnMut(QueueOwnerMessage, bool) -> Result<Option<TResult>, QueueConnectionError>,
    FClose: FnMut(bool) -> Result<Option<TResult>, QueueConnectionError>,
{
    let Some(mut socket) = connect_to_queue_owner(owner, None).await? else {
        return Ok(None);
    };

    let payload = serde_json::to_vec(request).map_err(|error| {
        make_queue_connection_error_with_source(
            "Failed to serialize queue request",
            "QUEUE_REQUEST_INVALID",
            false,
            error,
        )
    })?;
    socket.write_all(&payload).await.map_err(|error| {
        make_queue_connection_error_with_source(
            format!("Failed to write queue request: {error}"),
            "QUEUE_DISCONNECTED_BEFORE_ACK",
            true,
            error,
        )
    })?;
    socket.write_all(b"\n").await.map_err(|error| {
        make_queue_connection_error_with_source(
            format!("Failed to write queue request terminator: {error}"),
            "QUEUE_DISCONNECTED_BEFORE_ACK",
            true,
            error,
        )
    })?;

    let mut reader = BufReader::new(socket);
    let mut acknowledged = false;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await.map_err(|error| {
            make_queue_connection_error_with_source(
                format!("Queue owner read failed: {error}"),
                if acknowledged {
                    "QUEUE_DISCONNECTED_BEFORE_COMPLETION"
                } else {
                    "QUEUE_DISCONNECTED_BEFORE_ACK"
                },
                true,
                error,
            )
        })?;

        if bytes_read == 0 {
            return on_close(acknowledged);
        }
        if line.len() > MAX_MESSAGE_BUFFER_SIZE {
            return Err(make_queue_connection_error(
                format!("Message buffer exceeded {MAX_MESSAGE_BUFFER_SIZE} bytes"),
                "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
                true,
            ));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let message = parse_queue_owner_response_line(owner, queue_request_id(request), trimmed)?;
        match message {
            QueueOwnerMessage::Accepted { .. } => {
                acknowledged = true;
                if let Some(result) = on_accepted()? {
                    return Ok(Some(result));
                }
            }
            other => {
                if let Some(result) = on_message(other, acknowledged)? {
                    return Ok(Some(result));
                }
            }
        }
    }
}

fn queue_request_id(request: &QueueRequest) -> &str {
    match request {
        QueueRequest::SubmitPrompt { request_id, .. }
        | QueueRequest::CancelPrompt { request_id, .. }
        | QueueRequest::SetMode { request_id, .. }
        | QueueRequest::SetModel { request_id, .. }
        | QueueRequest::SetConfigOption { request_id, .. } => request_id,
    }
}

pub struct SubmitToQueueOwnerOptions<'a> {
    pub session_id: &'a str,
    pub message: String,
    pub prompt: Option<PromptInput>,
    pub permission_mode: PermissionMode,
    pub resume_policy: Option<SessionResumePolicy>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub output_formatter: &'a mut dyn OutputFormatter,
    pub error_emission_policy: Option<OutputErrorEmissionPolicy>,
    pub timeout_ms: Option<u64>,
    pub suppress_sdk_console_errors: Option<bool>,
    pub wait_for_completion: bool,
    pub verbose: bool,
}

#[allow(clippy::result_large_err)]
async fn submit_to_queue_owner(
    owner: &QueueOwnerRecord,
    options: &mut SubmitToQueueOwnerOptions<'_>,
) -> Result<Option<SessionSendOutcome>, QueueConnectionError> {
    let request_id = next_queue_request_id();
    let request = QueueRequest::SubmitPrompt {
        request_id: request_id.clone(),
        owner_generation: Some(owner.owner_generation),
        message: options.message.clone(),
        prompt: options
            .prompt
            .clone()
            .unwrap_or_else(|| crate::text_prompt(options.message.clone())),
        permission_mode: options.permission_mode,
        resume_policy: options.resume_policy,
        non_interactive_permissions: options.non_interactive_permissions,
        timeout_ms: options.timeout_ms,
        suppress_sdk_console_errors: options.suppress_sdk_console_errors,
        wait_for_completion: options.wait_for_completion,
    };

    options
        .output_formatter
        .set_context(OutputFormatterContext { session_id: options.session_id.to_string() });

    run_queue_owner_request(
        owner,
        &request,
        || {
            if !options.wait_for_completion {
                return Ok(Some(SessionSendOutcome::SessionEnqueueResult(SessionEnqueueResult {
                    queued: true,
                    session_id: options.session_id.to_string(),
                    request_id: request_id.clone(),
                })));
            }
            Ok(None)
        },
        |message, acknowledged| match message {
            QueueOwnerMessage::Error {
                code,
                detail_code,
                origin,
                message,
                retryable,
                acp,
                output_already_emitted,
                ..
            } => {
                options.output_formatter.set_context(OutputFormatterContext {
                    session_id: options.session_id.to_string(),
                });

                let queue_error_already_emitted = options
                    .error_emission_policy
                    .as_ref()
                    .map(|policy| policy.queue_error_already_emitted)
                    .unwrap_or(true);
                let should_emit_in_formatter =
                    !output_already_emitted.unwrap_or(false) || !queue_error_already_emitted;

                if should_emit_in_formatter {
                    options.output_formatter.on_error(OutputErrorParams {
                        code,
                        detail_code: detail_code.clone(),
                        origin: Some(origin),
                        message: message.clone(),
                        retryable,
                        acp: acp.clone(),
                        timestamp: None,
                    });
                    options.output_formatter.flush();
                }

                Err(QueueConnectionError::new(
                    message,
                    AcpxErrorOptions {
                        output_code: Some(code),
                        detail_code,
                        origin: Some(origin),
                        retryable,
                        acp,
                        output_already_emitted: queue_error_already_emitted,
                        ..AcpxErrorOptions::default()
                    },
                ))
            }
            QueueOwnerMessage::Event { message, .. } => {
                if !acknowledged {
                    return Err(make_queue_connection_error(
                        "Queue owner did not acknowledge request",
                        "QUEUE_ACK_MISSING",
                        true,
                    ));
                }
                options.output_formatter.on_acp_message(message);
                Ok(None)
            }
            QueueOwnerMessage::Result { result, .. } => {
                if !acknowledged {
                    return Err(make_queue_connection_error(
                        "Queue owner did not acknowledge request",
                        "QUEUE_ACK_MISSING",
                        true,
                    ));
                }
                options.output_formatter.flush();
                Ok(Some(SessionSendOutcome::SessionSendResult(result)))
            }
            _ => Err(make_queue_connection_error(
                "Queue owner returned unexpected response",
                "QUEUE_PROTOCOL_UNEXPECTED_RESPONSE",
                true,
            )),
        },
        |acknowledged| {
            if !acknowledged {
                return Err(make_queue_connection_error(
                    "Queue owner disconnected before acknowledging request",
                    "QUEUE_DISCONNECTED_BEFORE_ACK",
                    true,
                ));
            }
            if !options.wait_for_completion {
                return Ok(Some(SessionSendOutcome::SessionEnqueueResult(SessionEnqueueResult {
                    queued: true,
                    session_id: options.session_id.to_string(),
                    request_id: request_id.clone(),
                })));
            }
            Err(make_queue_connection_error(
                "Queue owner disconnected before prompt completion",
                "QUEUE_DISCONNECTED_BEFORE_COMPLETION",
                true,
            ))
        },
    )
    .await
}

#[allow(clippy::result_large_err)]
async fn submit_control_to_queue_owner<TResponse>(
    owner: &QueueOwnerRecord,
    request: &QueueRequest,
    expected: fn(QueueOwnerMessage) -> Option<TResponse>,
) -> Result<Option<TResponse>, QueueConnectionError> {
    run_queue_owner_request(
        owner,
        request,
        || Ok(None),
        |message, acknowledged| match message {
            QueueOwnerMessage::Error {
                code, detail_code, origin, message, retryable, acp, ..
            } => Err(QueueConnectionError::new(
                message,
                AcpxErrorOptions {
                    output_code: Some(code),
                    detail_code,
                    origin: Some(origin),
                    retryable,
                    acp,
                    ..AcpxErrorOptions::default()
                },
            )),
            other => {
                if !acknowledged {
                    return Err(make_queue_connection_error(
                        "Queue owner did not acknowledge request",
                        "QUEUE_ACK_MISSING",
                        true,
                    ));
                }
                if let Some(response) = expected(other) {
                    Ok(Some(response))
                } else {
                    Err(make_queue_connection_error(
                        "Queue owner returned unexpected response",
                        "QUEUE_PROTOCOL_UNEXPECTED_RESPONSE",
                        true,
                    ))
                }
            }
        },
        |acknowledged| {
            if !acknowledged {
                return Err(make_queue_connection_error(
                    "Queue owner disconnected before acknowledging request",
                    "QUEUE_DISCONNECTED_BEFORE_ACK",
                    true,
                ));
            }
            Err(make_queue_connection_error(
                "Queue owner disconnected before responding",
                "QUEUE_DISCONNECTED_BEFORE_COMPLETION",
                true,
            ))
        },
    )
    .await
}

#[allow(clippy::result_large_err)]
async fn submit_cancel_to_queue_owner(
    owner: &QueueOwnerRecord,
) -> Result<Option<bool>, QueueConnectionError> {
    let request_id = next_queue_request_id();
    let request = QueueRequest::CancelPrompt {
        request_id: request_id.clone(),
        owner_generation: Some(owner.owner_generation),
    };
    let response = submit_control_to_queue_owner(owner, &request, |message| match message {
        QueueOwnerMessage::CancelResult { request_id, cancelled, .. } => {
            Some((request_id, cancelled))
        }
        _ => None,
    })
    .await?;

    match response {
        Some((response_request_id, cancelled)) if response_request_id == request_id => {
            Ok(Some(cancelled))
        }
        Some(_) => Err(make_queue_connection_error(
            "Queue owner returned mismatched cancel response",
            "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
            true,
        )),
        None => Ok(None),
    }
}

async fn submit_set_mode_to_queue_owner(
    owner: &QueueOwnerRecord,
    mode_id: &str,
    timeout_ms: Option<u64>,
) -> Result<Option<bool>, QueueConnectionError> {
    let request_id = next_queue_request_id();
    let request = QueueRequest::SetMode {
        request_id: request_id.clone(),
        owner_generation: Some(owner.owner_generation),
        mode_id: mode_id.to_string(),
        timeout_ms,
    };
    let response = submit_control_to_queue_owner(owner, &request, |message| match message {
        QueueOwnerMessage::SetModeResult { request_id, .. } => Some(request_id),
        _ => None,
    })
    .await?;

    match response {
        Some(response_request_id) if response_request_id == request_id => Ok(Some(true)),
        Some(_) => Err(make_queue_connection_error(
            "Queue owner returned mismatched set_mode response",
            "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
            true,
        )),
        None => Ok(None),
    }
}

async fn submit_set_model_to_queue_owner(
    owner: &QueueOwnerRecord,
    model_id: &str,
    timeout_ms: Option<u64>,
) -> Result<Option<bool>, QueueConnectionError> {
    let request_id = next_queue_request_id();
    let request = QueueRequest::SetModel {
        request_id: request_id.clone(),
        owner_generation: Some(owner.owner_generation),
        model_id: model_id.to_string(),
        timeout_ms,
    };
    let response = submit_control_to_queue_owner(owner, &request, |message| match message {
        QueueOwnerMessage::SetModelResult { request_id, .. } => Some(request_id),
        _ => None,
    })
    .await?;

    match response {
        Some(response_request_id) if response_request_id == request_id => Ok(Some(true)),
        Some(_) => Err(make_queue_connection_error(
            "Queue owner returned mismatched set_model response",
            "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
            true,
        )),
        None => Ok(None),
    }
}

async fn submit_set_config_option_to_queue_owner(
    owner: &QueueOwnerRecord,
    config_id: &str,
    value: &str,
    timeout_ms: Option<u64>,
) -> Result<Option<SetSessionConfigOptionResponse>, QueueConnectionError> {
    let request_id = next_queue_request_id();
    let request = QueueRequest::SetConfigOption {
        request_id: request_id.clone(),
        owner_generation: Some(owner.owner_generation),
        config_id: config_id.to_string(),
        value: value.to_string(),
        timeout_ms,
    };
    let response = submit_control_to_queue_owner(owner, &request, |message| match message {
        QueueOwnerMessage::SetConfigOptionResult { request_id, response, .. } => {
            Some((request_id, response))
        }
        _ => None,
    })
    .await?;

    match response {
        Some((response_request_id, response)) if response_request_id == request_id => {
            Ok(Some(response))
        }
        Some(_) => Err(make_queue_connection_error(
            "Queue owner returned mismatched set_config_option response",
            "QUEUE_PROTOCOL_MALFORMED_MESSAGE",
            true,
        )),
        None => Ok(None),
    }
}

fn log_verbose(message: impl Display, verbose: bool) {
    if verbose {
        eprintln!("{message}");
    }
}

pub async fn try_submit_to_running_owner(
    options: &mut SubmitToQueueOwnerOptions<'_>,
) -> Result<Option<SessionSendOutcome>, QueueConnectionError> {
    let Some(owner) = read_default_queue_owner_record(options.session_id).await else {
        return Ok(None);
    };

    match submit_to_queue_owner(&owner, options).await {
        Ok(Some(submitted)) => {
            log_verbose(
                format!(
                    "[vwacp] queued prompt on active owner pid {} for session {}",
                    owner.pid, options.session_id
                ),
                options.verbose,
            );
            Ok(Some(submitted))
        }
        Ok(None) => {
            let health = probe_queue_owner_health(options.session_id).await;
            if !health.has_lease {
                return Ok(None);
            }
            Err(make_queue_connection_error(
                "Session queue owner is running but not accepting queue requests",
                "QUEUE_NOT_ACCEPTING_REQUESTS",
                true,
            ))
        }
        Err(error) => {
            if maybe_recover_stale_owner_after_protocol_mismatch(
                options.session_id,
                &owner,
                &error,
                options.verbose,
            )
            .await
            {
                return Ok(None);
            }
            Err(error)
        }
    }
}

pub async fn try_cancel_on_running_owner(
    session_id: &str,
    verbose: bool,
) -> Result<Option<bool>, QueueConnectionError> {
    let Some(owner) = read_default_queue_owner_record(session_id).await else {
        return Ok(None);
    };

    if let Some(cancelled) = submit_cancel_to_queue_owner(&owner).await? {
        log_verbose(
            format!(
                "[vwacp] requested cancel on active owner pid {} for session {}",
                owner.pid, session_id
            ),
            verbose,
        );
        return Ok(Some(cancelled));
    }

    let health = probe_queue_owner_health(session_id).await;
    if !health.has_lease {
        return Ok(None);
    }

    Err(make_queue_connection_error(
        "Session queue owner is running but not accepting cancel requests",
        "QUEUE_NOT_ACCEPTING_REQUESTS",
        true,
    ))
}

pub async fn try_set_mode_on_running_owner(
    session_id: &str,
    mode_id: &str,
    timeout_ms: Option<u64>,
    verbose: bool,
) -> Result<Option<bool>, QueueConnectionError> {
    let Some(owner) = read_default_queue_owner_record(session_id).await else {
        return Ok(None);
    };

    if let Some(result) = submit_set_mode_to_queue_owner(&owner, mode_id, timeout_ms).await? {
        log_verbose(
            format!(
                "[vwacp] requested session/set_mode on owner pid {} for session {}",
                owner.pid, session_id
            ),
            verbose,
        );
        return Ok(Some(result));
    }

    let health = probe_queue_owner_health(session_id).await;
    if !health.has_lease {
        return Ok(None);
    }

    Err(make_queue_connection_error(
        "Session queue owner is running but not accepting set_mode requests",
        "QUEUE_NOT_ACCEPTING_REQUESTS",
        true,
    ))
}

pub async fn try_set_model_on_running_owner(
    session_id: &str,
    model_id: &str,
    timeout_ms: Option<u64>,
    verbose: bool,
) -> Result<Option<bool>, QueueConnectionError> {
    let Some(owner) = read_default_queue_owner_record(session_id).await else {
        return Ok(None);
    };

    if let Some(result) = submit_set_model_to_queue_owner(&owner, model_id, timeout_ms).await? {
        log_verbose(
            format!(
                "[vwacp] requested session/set_model on owner pid {} for session {}",
                owner.pid, session_id
            ),
            verbose,
        );
        return Ok(Some(result));
    }

    let health = probe_queue_owner_health(session_id).await;
    if !health.has_lease {
        return Ok(None);
    }

    Err(make_queue_connection_error(
        "Session queue owner is running but not accepting set_model requests",
        "QUEUE_NOT_ACCEPTING_REQUESTS",
        true,
    ))
}

pub async fn try_set_config_option_on_running_owner(
    session_id: &str,
    config_id: &str,
    value: &str,
    timeout_ms: Option<u64>,
    verbose: bool,
) -> Result<Option<SetSessionConfigOptionResponse>, QueueConnectionError> {
    let Some(owner) = read_default_queue_owner_record(session_id).await else {
        return Ok(None);
    };

    if let Some(response) =
        submit_set_config_option_to_queue_owner(&owner, config_id, value, timeout_ms).await?
    {
        log_verbose(
            format!(
                "[vwacp] requested session/set_config_option on owner pid {} for session {}",
                owner.pid, session_id
            ),
            verbose,
        );
        return Ok(Some(response));
    }

    let health = probe_queue_owner_health(session_id).await;
    if !health.has_lease {
        return Ok(None);
    }

    Err(make_queue_connection_error(
        "Session queue owner is running but not accepting set_config_option requests",
        "QUEUE_NOT_ACCEPTING_REQUESTS",
        true,
    ))
}

#[cfg(test)]
#[path = "queue_ipc_tests.rs"]
mod queue_ipc_tests;
