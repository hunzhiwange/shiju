//! 聊天流式接口支持。
//!
//! 本模块负责与网关的 SSE 聊天流接口交互，包括：
//! - 发起流式聊天请求
//! - 解析 SSE 帧
//! - 将 JSON 事件映射为稳定的事件枚举
//! - 为异步与阻塞场景提供统一行为

use tracing::{debug, error, info};

use crate::endpoint::GatewayEndpoint;

/// 直接透出新的流式聊天事件类型。
pub use vw_api_types::chat::{GatewayChatStreamEvent, GatewayChatStreamRequest};

fn stream_http_failure_message(status: impl std::fmt::Display, body: &str) -> String {
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

/// 将网关 usage 载荷规整后的轻量统计结构。
///
/// 当前网关的 `chat.step_finish` 与 `chat.done` 都会携带这组字段；
/// 在 CLI/TUI 侧先统一成稳定结构，避免继续裸用 JSON。
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    #[serde(default)]
    pub reasoning_tokens: i64,
}

/// `chat.step_start` 的 typed 事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatStepStartEvent {
    pub step_index: u32,
    pub created_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// `chat.step_finish` 的 typed 事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatStepFinishEvent {
    pub step_index: u32,
    pub finished_ms: u64,
    #[serde(default)]
    pub usage: GatewayChatUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// `chat.post_tool_round` 的 typed 事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatPostToolRoundEvent {
    pub step_index: u32,
}

/// `chat.tool` 生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayChatToolEventKind {
    Start,
    Update,
    Complete,
}

/// `chat.tool` 的协议中立状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayChatToolStatus {
    Running,
    Waiting,
    Completed,
    Failed,
    Denied,
    Cancelled,
    Other(String),
}

/// 网关规整后的类型化工具生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayChatToolEvent {
    pub step_index: u32,
    pub call_id: String,
    pub name: String,
    pub kind: GatewayChatToolEventKind,
    pub status: GatewayChatToolStatus,
    pub occurred_ms: u64,
    pub payload: serde_json::Value,
}

impl GatewayChatToolEvent {
    /// 保留旧聊天工具卡消费的文本协议。
    pub fn legacy_delta(&self) -> String {
        format!("tool {}\n{}\n", self.name, self.payload)
    }
}

/// 网关流事件经客户端规整后的 typed 结果。
///
/// `GatewayChatStreamEvent` 仍保留原始兼容接口；新调用点应尽量改用
/// 这里的 typed 结果，避免直接在 UI 层解析 `Other(JSON)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayTypedChatStreamEvent {
    Delta(String),
    ContextLoaded {
        paths: Vec<String>,
    },
    StepStart(GatewayChatStepStartEvent),
    StepFinish(GatewayChatStepFinishEvent),
    PostToolRound(GatewayChatPostToolRoundEvent),
    TodoUpdated {
        session_id: Option<String>,
    },
    QuestionRaised {
        session_id: Option<String>,
    },
    QuestionResolved {
        session_id: Option<String>,
    },
    UsageUpdated {
        session_id: Option<String>,
        usage: GatewayChatUsage,
    },
    TitleUpdated {
        session_id: Option<String>,
        title: String,
    },
    SessionUpdated {
        session_id: Option<String>,
        title: Option<String>,
    },
    Done {
        finish_reason: Option<String>,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    Error(String),
    Unknown {
        event_type: Option<String>,
    },
}

/// 将 `chat.tool` JSON 规整为封闭的类型化工具事件。
pub fn normalize_chat_tool_event(payload: &serde_json::Value) -> Option<GatewayChatToolEvent> {
    if payload.get("type").and_then(serde_json::Value::as_str) != Some("chat.tool") {
        return None;
    }

    let step_index = u32::try_from(payload.get("step_index")?.as_u64()?).ok()?;
    let call_id = payload.get("call_id")?.as_str()?.to_string();
    let name = payload.get("name")?.as_str()?.to_string();
    let kind = match payload.get("kind")?.as_str()? {
        "start" => GatewayChatToolEventKind::Start,
        "update" => GatewayChatToolEventKind::Update,
        "complete" => GatewayChatToolEventKind::Complete,
        _ => return None,
    };
    let status = match payload.get("status")?.as_str()? {
        "running" => GatewayChatToolStatus::Running,
        "waiting" => GatewayChatToolStatus::Waiting,
        "completed" => GatewayChatToolStatus::Completed,
        "failed" => GatewayChatToolStatus::Failed,
        "denied" => GatewayChatToolStatus::Denied,
        "cancelled" => GatewayChatToolStatus::Cancelled,
        other => GatewayChatToolStatus::Other(other.to_string()),
    };
    let occurred_ms = payload.get("occurred_ms")?.as_u64()?;
    let event_payload = payload.get("payload")?.clone();

    Some(GatewayChatToolEvent {
        step_index,
        call_id,
        name,
        kind,
        status,
        occurred_ms,
        payload: event_payload,
    })
}

#[deprecated(note = "use GatewayChatStreamEvent")]
/// 兼容旧命名的流式聊天事件类型。
pub type ChatStreamEvent = GatewayChatStreamEvent;
#[deprecated(note = "use GatewayChatStreamRequest")]
/// 兼容旧命名的流式聊天请求类型。
pub type ChatStreamRequest = GatewayChatStreamRequest;

#[allow(dead_code)]
/// 建立流连接时的默认超时时间，单位为秒。
pub const STREAM_CONNECT_TIMEOUT_SECS: u64 = 30;
#[allow(dead_code)]
/// 单次流式请求允许持续的最长时间，单位为秒。
pub const STREAM_REQUEST_TIMEOUT_SECS: u64 = 60 * 60 * 3;
/// 标识同一次可恢复流运行的请求头。
pub const STREAM_REQUEST_ID_HEADER: &str = "x-vw-stream-request-id";
/// 指示服务端只回放该序号之后事件的标准 SSE 请求头。
pub const LAST_EVENT_ID_HEADER: &str = "last-event-id";
const CHAT_STREAM_PATH: &str = "/v1/chat/stream";

/// 单次可恢复流请求携带的稳定运行标识与已确认游标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayStreamResume {
    pub request_id: String,
    pub last_event_id: Option<u64>,
}

/// 带服务端单调 SSE 序号的业务事件。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayStreamEvent<T> {
    pub id: u64,
    pub event: T,
}

/// 通用 Gateway SSE 帧，保留可选 id、event 名和 JSON data。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GatewayJsonSseEvent {
    pub id: Option<u64>,
    pub event: Option<String>,
    pub data: serde_json::Value,
}

/// 订阅固定 Gateway JSON SSE 路径，按 wire 顺序回调事件。
pub(crate) async fn stream_json_sse(
    endpoint: &GatewayEndpoint,
    path: &str,
    query: &[(String, String)],
    mut on_event: impl FnMut(GatewayJsonSseEvent) -> bool,
) -> Result<GatewayStreamAttemptResult, String> {
    use futures_util::StreamExt;

    use crate::http::{apply_request_context, log_request, response_error, transport_error};

    log_request::<serde_json::Value>("GET", endpoint, path, query, None);
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|error| transport_error("GET", endpoint, path, error))?;
    let request = apply_request_context(
        client.get(format!("{}{}", endpoint.base_url(), path)).query(query),
        endpoint,
    );
    let response = request
        .send()
        .await
        .map_err(|error| transport_error("GET", endpoint, path, error))?;
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

/// 一次可恢复流订阅尝试的结束原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayStreamAttemptResult {
    /// 已收到业务协议定义的明确终态。
    Terminated,
    /// 调用方拒绝继续接收事件。
    CallerStopped,
    /// 连接在明确终态之前结束，调用方可使用原请求标识和游标重连。
    Incomplete,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GatewaySseFrame {
    pub(crate) id: Option<u64>,
    pub(crate) data: String,
}

/// 将原始网关流事件规整为 typed 结果。
///
/// 当前只补齐新 TUI 已确认会消费的基础事件：
/// - delta
/// - context_loaded
/// - step_start / step_finish
/// - done / error
///
/// 未知事件会降级为 `Unknown`，但不会再把原始 JSON 暴露给 CLI runtime。
pub fn normalize_chat_stream_event(event: GatewayChatStreamEvent) -> GatewayTypedChatStreamEvent {
    match event {
        GatewayChatStreamEvent::Delta(delta) => GatewayTypedChatStreamEvent::Delta(delta),
        GatewayChatStreamEvent::Done { finish_reason, usage, message_id, parent_message_id } => {
            GatewayTypedChatStreamEvent::Done {
                finish_reason,
                usage: normalize_chat_usage(usage),
                message_id,
                parent_message_id,
            }
        }
        GatewayChatStreamEvent::Error(error) => GatewayTypedChatStreamEvent::Error(error),
        GatewayChatStreamEvent::Other(payload) => {
            match payload.get("type").and_then(serde_json::Value::as_str) {
                Some("chat.context_loaded") => GatewayTypedChatStreamEvent::ContextLoaded {
                    paths: payload
                        .get("paths")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|path| !path.is_empty())
                        .map(ToOwned::to_owned)
                        .collect(),
                },
                Some("chat.step_start") => {
                    GatewayTypedChatStreamEvent::StepStart(parse_step_start_payload(&payload))
                }
                Some("chat.step_finish") => {
                    GatewayTypedChatStreamEvent::StepFinish(parse_step_finish_payload(&payload))
                }
                Some("chat.post_tool_round") => GatewayTypedChatStreamEvent::PostToolRound(
                    parse_post_tool_round_payload(&payload),
                ),
                Some("chat.todo_updated") => GatewayTypedChatStreamEvent::TodoUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                Some("chat.question_raised") => GatewayTypedChatStreamEvent::QuestionRaised {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                Some("chat.question_resolved") => GatewayTypedChatStreamEvent::QuestionResolved {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                Some("chat.usage_updated") => GatewayTypedChatStreamEvent::UsageUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    usage: payload
                        .get("usage")
                        .cloned()
                        .and_then(|value| serde_json::from_value::<GatewayChatUsage>(value).ok())
                        .unwrap_or_default(),
                },
                Some("chat.title_updated") => GatewayTypedChatStreamEvent::TitleUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    title: payload
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                },
                Some("chat.session_updated") => GatewayTypedChatStreamEvent::SessionUpdated {
                    session_id: payload
                        .get("session_id")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    title: payload
                        .get("session")
                        .and_then(|session| session.get("title"))
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                },
                other => GatewayTypedChatStreamEvent::Unknown {
                    event_type: other.map(ToOwned::to_owned),
                },
            }
        }
    }
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

    use crate::http::{apply_request_context, directory_query, log_request, transport_error};

    log_request("POST", endpoint, CHAT_STREAM_PATH, &directory_query(directory), Some(body));

    #[cfg(not(target_arch = "wasm32"))]
    let builder = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS));
    #[cfg(target_arch = "wasm32")]
    let builder = reqwest::Client::builder();

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

    use crate::http::{
        apply_blocking_request_context, directory_query, log_request, transport_error,
    };

    log_request("POST", endpoint, CHAT_STREAM_PATH, &directory_query(directory), Some(body));
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(STREAM_CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|err| transport_error("POST", endpoint, CHAT_STREAM_PATH, err))?;
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

/// 将网关返回的原始 JSON 事件映射为稳定的流式事件枚举。
pub(crate) fn parse_stream_event(payload: serde_json::Value) -> GatewayChatStreamEvent {
    match payload.get("type").and_then(serde_json::Value::as_str).unwrap_or_default() {
        "chat.delta" => GatewayChatStreamEvent::Delta(
            payload
                .get("delta")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "chat.done" => GatewayChatStreamEvent::Done {
            finish_reason: payload
                .get("finish_reason")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            usage: payload.get("usage").cloned(),
            message_id: payload
                .get("message_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            parent_message_id: payload
                .get("parent_message_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        },
        "chat.error" => GatewayChatStreamEvent::Error(
            payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("gateway stream failed")
                .to_string(),
        ),
        _ => GatewayChatStreamEvent::Other(payload),
    }
}

/// 从缓冲区中提取一个完整的 SSE 帧，保留未消费部分供后续继续解析。
pub(crate) fn take_next_sse_event(buffer: &mut String) -> Option<String> {
    let separator_len = if let Some(idx) = buffer.find("\r\n\r\n") {
        Some((idx, 4))
    } else {
        buffer.find("\n\n").map(|idx| (idx, 2))
    }?;

    let (idx, sep_len) = separator_len;
    let frame = buffer[..idx].to_string();
    buffer.drain(..idx + sep_len);
    Some(frame)
}

pub(crate) fn parse_sse_frame(frame: &str) -> Result<Option<GatewaySseFrame>, String> {
    let mut id = None;
    let data = frame
        .lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            if let Some(value) = line.strip_prefix("id:") {
                id = Some(value.trim().parse::<u64>().map_err(|_| {
                    "gateway stream SSE id must be a non-negative integer".to_string()
                }));
                None
            } else {
                line.strip_prefix("data:").map(|value| Ok(value.trim_start()))
            }
        })
        .collect::<Result<Vec<_>, String>>()?
        .join("\n");

    if data.is_empty() {
        return Ok(None);
    }
    Ok(Some(GatewaySseFrame { id: id.transpose()?, data }))
}

pub(crate) fn apply_stream_resume(
    mut request: reqwest::RequestBuilder,
    resume: &GatewayStreamResume,
) -> reqwest::RequestBuilder {
    request = request.header(STREAM_REQUEST_ID_HEADER, &resume.request_id);
    if let Some(last_event_id) = resume.last_event_id {
        request = request.header(LAST_EVENT_ID_HEADER, last_event_id);
    }
    request
}

fn required_event_id(id: Option<u64>, resumable: bool) -> Result<u64, String> {
    match (id, resumable) {
        (Some(id), _) => Ok(id),
        (None, true) => Err("gateway resumable stream event is missing SSE id".to_string()),
        (None, false) => Ok(0),
    }
}

/// 返回事件对应的稳定日志名称，便于统计与调试。
pub(crate) fn event_name(event: &GatewayChatStreamEvent) -> &'static str {
    match event {
        GatewayChatStreamEvent::Delta(_) => "chat.delta",
        GatewayChatStreamEvent::Done { .. } => "chat.done",
        GatewayChatStreamEvent::Error(_) => "chat.error",
        GatewayChatStreamEvent::Other(_) => "other",
    }
}

fn is_terminal_stream_event(event: &GatewayChatStreamEvent) -> bool {
    matches!(event, GatewayChatStreamEvent::Done { .. } | GatewayChatStreamEvent::Error(_))
}

fn normalize_chat_usage(value: Option<serde_json::Value>) -> Option<GatewayChatUsage> {
    value.and_then(|value| serde_json::from_value::<GatewayChatUsage>(value).ok())
}

fn parse_step_start_payload(payload: &serde_json::Value) -> GatewayChatStepStartEvent {
    GatewayChatStepStartEvent {
        step_index: payload
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default(),
        created_ms: payload
            .get("created_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        model: payload.get("model").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
    }
}

fn parse_step_finish_payload(payload: &serde_json::Value) -> GatewayChatStepFinishEvent {
    GatewayChatStepFinishEvent {
        step_index: payload
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default(),
        finished_ms: payload
            .get("finished_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        usage: payload
            .get("usage")
            .cloned()
            .and_then(|value| serde_json::from_value::<GatewayChatUsage>(value).ok())
            .unwrap_or_default(),
        finish_reason: payload
            .get("finish_reason")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        model: payload.get("model").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
    }
}

fn parse_post_tool_round_payload(payload: &serde_json::Value) -> GatewayChatPostToolRoundEvent {
    GatewayChatPostToolRoundEvent {
        step_index: payload
            .get("step_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
