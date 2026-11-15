//! 聊天与通用 JSON SSE 网络订阅。

use tracing::{debug, error, info};
use gateway_endpoint::GatewayEndpoint;
use crate::types::*;
use crate::sse::*;

const CHAT_STREAM_PATH: &str = "/v1/chat/stream";

pub(crate) fn stream_http_failure_message(status: impl std::fmt::Display, body: &str) -> String {
    let status_text = status.to_string();
    let body = body.trim();
    let is_payload_too_large = status_text.starts_with("413")
        || body.contains("length limit exceeded")
        || body.to_ascii_lowercase().contains("payload too large");
    if is_payload_too_large {
        return "请求内容过大（图片或会话历史超出网关限制），请压缩图片、减少附件数量或开启新会话后重试"
            .to_string();
    }
    if body.is_empty() {
        format!("gateway stream failed: {status_text}")
    } else {
        format!("gateway stream failed: {status_text} {body}")
    }
}

/// 订阅固定 Gateway JSON SSE 路径，按 wire 顺序回调事件。
pub async fn stream_json_sse(
    endpoint: &GatewayEndpoint,
    path: &str,
    query: &[(String, String)],
    mut on_event: impl FnMut(GatewayJsonSseEvent) -> bool,
) -> Result<GatewayStreamAttemptResult, String> {
    use futures_util::StreamExt;

    use gateway_http::{apply_request_context, log_request, response_error, transport_error};

    log_request::<serde_json::Value>("GET", endpoint, path, query, None);
    #[cfg(not(target_arch = "wasm32"))]
    let builder = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS));
    #[cfg(target_arch = "wasm32")]
    let builder = reqwest::Client::builder();
    #[cfg(not(target_arch = "wasm32"))]
    let builder = if endpoint.is_loopback() { builder.no_proxy() } else { builder };
    let client = builder.build().map_err(|error| transport_error("GET", endpoint, path, error))?;
    let request = apply_request_context(
        client.get(format!("{}{}", endpoint.base_url(), path)).query(query),
        endpoint,
    );
    let response =
        request.send().await.map_err(|error| transport_error("GET", endpoint, path, error))?;
    if !response.status().is_success() {
        return Err(response_error("GET", endpoint, path, response).await);
    }

    let mut bytes_stream = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = bytes_stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(raw_frame) = take_next_sse_event(&mut buffer) {
            let Some(frame) = parse_sse_frame(&raw_frame)? else {
                continue;
            };
            let event_name = raw_frame
                .lines()
                .filter_map(|line| line.trim_end_matches('\r').strip_prefix("event:"))
                .map(str::trim)
                .find(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let data = serde_json::from_str(&frame.data).map_err(|error| error.to_string())?;
            if !on_event(GatewayJsonSseEvent { id: frame.id, event: event_name, data }) {
                return Ok(GatewayStreamAttemptResult::CallerStopped);
            }
        }
    }
    Ok(GatewayStreamAttemptResult::Incomplete)
}

/// 连接网关聊天流接口，并按收到的 SSE 事件顺序回调消费者。
///
/// # 参数
///
/// - `endpoint`: 目标网关地址与认证信息
/// - `directory`: 可选目录上下文，会透传给网关
/// - `body`: 聊天流请求体
/// - `on_event`: 事件消费回调，返回 `false` 表示提前停止
///
/// # 行为说明
///
/// 该函数会持续读取 SSE 数据块，并在解析出完整帧后逐个回调调用方。
pub async fn stream_chat(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    on_event: impl FnMut(GatewayChatStreamEvent) -> bool,
) -> Result<(), String> {
    stream_chat_inner(endpoint, directory, body, None, on_event).await
}

/// 执行一次支持请求复用与游标恢复的聊天流订阅。
pub async fn stream_chat_resumable(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    resume: &GatewayStreamResume,
    on_event: impl FnMut(GatewayStreamEvent<GatewayChatStreamEvent>) -> bool,
) -> Result<GatewayStreamAttemptResult, String> {
    stream_chat_attempt(endpoint, directory, body, Some(resume), None, on_event).await
}

#[cfg(not(target_arch = "wasm32"))]
/// 连接网关聊天流接口，并在连续无响应超过 `idle_timeout` 时返回超时错误。
///
/// 该超时只约束两次网关响应之间的空窗，不限制整次任务的总执行时长。
pub async fn stream_chat_with_idle_timeout(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    idle_timeout: std::time::Duration,
    on_event: impl FnMut(GatewayChatStreamEvent) -> bool,
) -> Result<(), String> {
    stream_chat_inner(endpoint, directory, body, Some(idle_timeout), on_event).await
}

async fn stream_chat_inner(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    idle_timeout: Option<std::time::Duration>,
    mut on_event: impl FnMut(GatewayChatStreamEvent) -> bool,
) -> Result<(), String> {
    stream_chat_attempt(endpoint, directory, body, None, idle_timeout, |event| {
        on_event(event.event)
    })
    .await
    .map(|_| ())
}

async fn stream_chat_attempt(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    resume: Option<&GatewayStreamResume>,
    idle_timeout: Option<std::time::Duration>,
    mut on_event: impl FnMut(GatewayStreamEvent<GatewayChatStreamEvent>) -> bool,
) -> Result<GatewayStreamAttemptResult, String> {
    use futures_util::StreamExt;

    use gateway_http::{apply_request_context, directory_query, log_request, transport_error};

    log_request("POST", endpoint, CHAT_STREAM_PATH, &directory_query(directory), Some(body));

    #[cfg(not(target_arch = "wasm32"))]
    let builder = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(STREAM_CONNECT_TIMEOUT_SECS));
    #[cfg(not(target_arch = "wasm32"))]
    let builder = if chat_uses_acp(body) {
        builder
    } else {
        builder.timeout(std::time::Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS))
    };
    #[cfg(target_arch = "wasm32")]
    let builder = reqwest::Client::builder();

    #[cfg(not(target_arch = "wasm32"))]
    let builder = if endpoint.is_loopback() { builder.no_proxy() } else { builder };
    let client =
        builder.build().map_err(|err| transport_error("POST", endpoint, CHAT_STREAM_PATH, err))?;
    let mut request =
        client.post(format!("{}{}", endpoint.base_url(), CHAT_STREAM_PATH)).json(body);
    if let Some(directory) = directory.filter(|value| !value.trim().is_empty()) {
        request = request.query(&[("directory", directory)]);
    }
    if let Some(resume) = resume {
        request = apply_stream_resume(request, resume);
    }
    request = apply_request_context(request, endpoint);

    let response = request
        .send()
        .await
        .map_err(|err| transport_error("POST", endpoint, CHAT_STREAM_PATH, err))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        error!(
            target: "vw_gateway_client",
            method = "POST",
            endpoint = %endpoint.describe(),
            path = CHAT_STREAM_PATH,
            status = %status,
            response_body = %body.trim(),
            "gateway stream request failed"
        );
        return Err(stream_http_failure_message(status, &body));
    }

    info!(
        target: "vw_gateway_client",
        method = "POST",
        endpoint = %endpoint.describe(),
        path = CHAT_STREAM_PATH,
        "gateway stream connected"
    );

    let mut bytes_stream = response.bytes_stream();
    let mut sse_buffer = String::new();
    let mut last_delivered_id = resume.and_then(|resume| resume.last_event_id).unwrap_or(0);

    loop {
        #[cfg(not(target_arch = "wasm32"))]
        let next_chunk = if let Some(timeout) = idle_timeout {
            tokio::time::timeout(timeout, bytes_stream.next()).await.map_err(|_| {
                format!("gateway stream idle timeout after {timeout:?} without response")
            })?
        } else {
            bytes_stream.next().await
        };
        #[cfg(target_arch = "wasm32")]
        let next_chunk = {
            let _ = idle_timeout;
            bytes_stream.next().await
        };
        let Some(chunk) = next_chunk else {
            break;
        };
        let chunk = chunk.map_err(|err| err.to_string())?;
        sse_buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(frame) = take_next_sse_event(&mut sse_buffer) {
            let Some(frame) = parse_sse_frame(&frame)? else {
                continue;
            };
            let id = required_event_id(frame.id, resume.is_some())?;
            if resume.is_some() {
                if id <= last_delivered_id {
                    continue;
                }
                last_delivered_id = id;
            }

            let payload: serde_json::Value =
                serde_json::from_str(&frame.data).map_err(|err| err.to_string())?;
            let event = parse_stream_event(payload);
            debug!(
                target: "vw_gateway_client",
                endpoint = %endpoint.describe(),
                path = CHAT_STREAM_PATH,
                event_type = %event_name(&event),
                "gateway stream event"
            );
            let event_type = event_name(&event);
            let terminal_event = is_terminal_stream_event(&event);
            if !on_event(GatewayStreamEvent { id, event }) {
                info!(
                    target: "vw_gateway_client",
                    endpoint = %endpoint.describe(),
                    path = CHAT_STREAM_PATH,
                    reason = "consumer requested stop",
                    terminal_event = event_type,
                    "gateway stream finished"
                );
                return Ok(GatewayStreamAttemptResult::CallerStopped);
            }
            if terminal_event {
                info!(
                    target: "vw_gateway_client",
                    endpoint = %endpoint.describe(),
                    path = CHAT_STREAM_PATH,
                    reason = "terminal event",
                    terminal_event = event_type,
                    "gateway stream finished"
                );
                return Ok(GatewayStreamAttemptResult::Terminated);
            }
        }
    }

    info!(
        target: "vw_gateway_client",
        endpoint = %endpoint.describe(),
        path = CHAT_STREAM_PATH,
        reason = "server closed stream",
        "gateway stream finished"
    );

    Ok(GatewayStreamAttemptResult::Incomplete)
}

#[cfg(not(target_arch = "wasm32"))]
fn chat_uses_acp(body: &GatewayChatStreamRequest) -> bool {
    let options = body.options.as_ref();
    body.acp_agent.as_deref().is_some_and(|agent| !agent.trim().is_empty())
        || body.model.as_deref().is_some_and(|model| model.starts_with("acp/"))
        || options.is_some_and(|options| {
            options["acp_test"] == true
                || options["acp_agent"].as_str().is_some_and(|agent| !agent.trim().is_empty())
        })
}

#[cfg(not(target_arch = "wasm32"))]
/// 以阻塞方式连接聊天流接口，适用于非异步调用场景。
///
/// 除了 I/O 模式不同，事件解析与终止条件都与异步版本保持一致。
pub fn stream_chat_blocking(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    mut on_event: impl FnMut(GatewayChatStreamEvent) -> bool,
) -> Result<(), String> {
    use std::io::Read;

    use gateway_http::{
        apply_blocking_request_context, directory_query, log_request, transport_error,
    };

    log_request("POST", endpoint, CHAT_STREAM_PATH, &directory_query(directory), Some(body));
    let builder = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(
            (!chat_uses_acp(body))
                .then(|| std::time::Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS)),
        );
    let builder = if endpoint.is_loopback() { builder.no_proxy() } else { builder };
    let client =
        builder.build().map_err(|err| transport_error("POST", endpoint, CHAT_STREAM_PATH, err))?;
    let mut request =
        client.post(format!("{}{}", endpoint.base_url(), CHAT_STREAM_PATH)).json(body);
    if let Some(directory) = directory.filter(|value| !value.trim().is_empty()) {
        request = request.query(&[("directory", directory)]);
    }
    request = apply_blocking_request_context(request, endpoint);

    let response =
        request.send().map_err(|err| transport_error("POST", endpoint, CHAT_STREAM_PATH, err))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        error!(
            target: "vw_gateway_client",
            method = "POST",
            endpoint = %endpoint.describe(),
            path = CHAT_STREAM_PATH,
            status = %status,
            response_body = %body.trim(),
            "gateway stream request failed"
        );
        return Err(stream_http_failure_message(status, &body));
    }

    info!(
        target: "vw_gateway_client",
        method = "POST",
        endpoint = %endpoint.describe(),
        path = CHAT_STREAM_PATH,
        "gateway stream connected"
    );

    let mut reader = response;
    let mut chunk = [0_u8; 4096];
    let mut sse_buffer = String::new();

    loop {
        let bytes_read = reader.read(&mut chunk).map_err(|err| err.to_string())?;
        if bytes_read == 0 {
            break;
        }

        sse_buffer.push_str(&String::from_utf8_lossy(&chunk[..bytes_read]));

        while let Some(frame) = take_next_sse_event(&mut sse_buffer) {
            let payload = frame
                .lines()
                .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
                .map(|line| line.trim_start())
                .collect::<Vec<_>>()
                .join("\n");

            if payload.is_empty() {
                continue;
            }

            let payload: serde_json::Value =
                serde_json::from_str(&payload).map_err(|err| err.to_string())?;
            let event = parse_stream_event(payload);
            debug!(
                target: "vw_gateway_client",
                endpoint = %endpoint.describe(),
                path = CHAT_STREAM_PATH,
                event_type = %event_name(&event),
                "gateway stream event"
            );
            let event_type = event_name(&event);
            let terminal_event = is_terminal_stream_event(&event);
            if !on_event(event) {
                info!(
                    target: "vw_gateway_client",
                    endpoint = %endpoint.describe(),
                    path = CHAT_STREAM_PATH,
                    reason = "consumer requested stop",
                    terminal_event = event_type,
                    "gateway stream finished"
                );
                return Ok(());
            }
            if terminal_event {
                info!(
                    target: "vw_gateway_client",
                    endpoint = %endpoint.describe(),
                    path = CHAT_STREAM_PATH,
                    reason = "terminal event",
                    terminal_event = event_type,
                    "gateway stream finished"
                );
                return Ok(());
            }
        }
    }

    info!(
        target: "vw_gateway_client",
        endpoint = %endpoint.describe(),
        path = CHAT_STREAM_PATH,
        reason = "server closed stream",
        "gateway stream finished"
    );

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// 以阻塞方式连接聊天流接口，并在连续无响应超过 `idle_timeout` 时返回超时错误。
pub fn stream_chat_blocking_with_idle_timeout(
    endpoint: &GatewayEndpoint,
    directory: Option<&str>,
    body: &GatewayChatStreamRequest,
    idle_timeout: std::time::Duration,
    on_event: impl FnMut(GatewayChatStreamEvent) -> bool,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| err.to_string())?;
    runtime.block_on(stream_chat_with_idle_timeout(
        endpoint,
        directory,
        body,
        idle_timeout,
        on_event,
    ))
}

pub fn apply_stream_resume(
    mut request: reqwest::RequestBuilder,
    resume: &GatewayStreamResume,
) -> reqwest::RequestBuilder {
    request = request.header(STREAM_REQUEST_ID_HEADER, &resume.request_id);
    if let Some(last_event_id) = resume.last_event_id {
        request = request.header(LAST_EVENT_ID_HEADER, last_event_id);
    }
    request
}
