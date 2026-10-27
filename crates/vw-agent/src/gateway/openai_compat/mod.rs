//! OpenAI 兼容的模型列表与聊天补全端点，负责请求映射和流式响应。
//!
//! 请求在进入代理提供者前转换为内部消息类型；响应保持 OpenAI JSON 或 SSE 事件形状。
//!
//! OpenAI-compatible `/v1/chat/completions` and `/v1/models` endpoints.
//!
//! These endpoints allow VibeWindow to act as a drop-in replacement for the
//! OpenAI API, enabling any OpenAI-compatible client (e.g., `openai` Python
//! library, `curl`, Aura) to send chat requests through the gateway.

use super::AppState;
use crate::app::agent::providers::traits::{ChatMessage, StreamOptions};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Instant;
use uuid::Uuid;

// ══════════════════════════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
/// OpenAI 兼容的聊天补全请求。
pub struct ChatCompletionsRequest {
    /// 模型标识；缺省时使用网关默认模型。
    #[serde(default)]
    pub model: Option<String>,
    /// 按对话顺序排列的消息。
    pub messages: Vec<ChatCompletionsMessage>,
    /// 采样温度；缺省时使用网关默认值。
    #[serde(default)]
    pub temperature: Option<f64>,
    /// 是否以 SSE 事件流返回响应。
    #[serde(default)]
    pub stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
/// OpenAI 兼容的单条聊天消息。
pub struct ChatCompletionsMessage {
    /// 消息角色。
    pub role: String,
    /// 纯文本消息内容。
    pub content: String,
}

#[derive(Debug, Serialize)]
/// 非流式聊天补全响应。
pub struct ChatCompletionsResponse {
    /// 本次补全的唯一标识。
    pub id: String,
    /// 固定的对象类型。
    pub object: &'static str,
    /// 创建时间的 Unix 秒数。
    pub created: u64,
    /// 实际使用的模型标识。
    pub model: String,
    /// 候选结果列表。
    pub choices: Vec<ChatCompletionsChoice>,
    /// 估算的令牌用量。
    pub usage: ChatCompletionsUsage,
}

#[derive(Debug, Serialize)]
/// 单个聊天补全候选。
pub struct ChatCompletionsChoice {
    /// 候选序号。
    pub index: u32,
    /// 助手响应消息。
    pub message: ChatCompletionsResponseMessage,
    /// 响应结束原因。
    pub finish_reason: &'static str,
}

#[derive(Debug, Serialize)]
/// 聊天补全响应中的助手消息。
pub struct ChatCompletionsResponseMessage {
    /// 固定的助手角色。
    pub role: &'static str,
    /// 模型生成的文本。
    pub content: String,
}

#[derive(Debug, Serialize)]
/// 聊天补全的估算令牌统计。
pub struct ChatCompletionsUsage {
    /// 输入消息估算令牌数。
    pub prompt_tokens: u32,
    /// 输出内容估算令牌数。
    pub completion_tokens: u32,
    /// 输入与输出令牌总数。
    pub total_tokens: u32,
}

/// SSE streaming chunk format.
#[derive(Debug, Serialize)]
struct ChatCompletionsChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Serialize)]
struct ChunkChoice {
    index: u32,
    delta: ChunkDelta,
    finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

fn chunk_bytes(
    id: String,
    created: u64,
    model: String,
    role: Option<&'static str>,
    content: Option<String>,
    finish_reason: Option<&'static str>,
    done: bool,
) -> axum::body::Bytes {
    let chunk = ChatCompletionsChunk {
        id,
        object: "chat.completion.chunk",
        created,
        model,
        choices: vec![ChunkChoice { index: 0, delta: ChunkDelta { role, content }, finish_reason }],
    };
    let json = serde_json::to_string(&chunk)
        .expect("chat completions chunk serialization should not fail");
    if done {
        axum::body::Bytes::from(format!("data: {json}\n\ndata: [DONE]\n\n"))
    } else {
        axum::body::Bytes::from(format!("data: {json}\n\n"))
    }
}

#[derive(Debug, Serialize)]
/// OpenAI 兼容的模型列表响应。
pub struct ModelsResponse {
    /// 固定的列表对象类型。
    pub object: &'static str,
    /// 可用模型条目。
    pub data: Vec<ModelObject>,
}

#[derive(Debug, Serialize)]
/// OpenAI 兼容的模型描述。
pub struct ModelObject {
    /// 模型标识。
    pub id: String,
    /// 固定的模型对象类型。
    pub object: &'static str,
    /// 条目创建时间的 Unix 秒数。
    pub created: u64,
    /// 模型所属提供者。
    pub owned_by: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// 处理 `/v1/chat/completions`，根据 `stream` 选择 JSON 或 SSE 响应。
pub async fn handle_v1_chat_completions(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // ── Rate limit ──
    let rate_key =
        super::client_key_from_request(Some(peer_addr), &headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_webhook(&rate_key) {
        tracing::warn!("/v1/chat/completions rate limit exceeded");
        let err = serde_json::json!({
            "error": {
                "message": "Rate limit exceeded. Please retry later.",
                "type": "rate_limit_error",
                "code": "rate_limit_exceeded"
            }
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err)).into_response();
    }

    // ── Gateway skey auth ──
    if state.pairing.auth_enabled() {
        let skey = super::api::auth::extract_auth_skey(&headers).unwrap_or("");
        if !state.pairing.is_authenticated(skey) {
            tracing::warn!("/v1/chat/completions: rejected — invalid gateway skey");
            let err = serde_json::json!({
                "error": {
                    "message": "Invalid API key. Send a valid skey as Authorization: Bearer <skey>.",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            });
            return (StatusCode::UNAUTHORIZED, Json(err)).into_response();
        }
    }

    // ── Parse body ──
    let request: ChatCompletionsRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!("/v1/chat/completions JSON parse error: {e}");
            let err = serde_json::json!({
                "error": {
                    "message": format!("Invalid JSON body: {e}"),
                    "type": "invalid_request_error",
                    "code": "invalid_json"
                }
            });
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();
        }
    };

    if request.messages.is_empty() {
        let err = serde_json::json!({
            "error": {
                "message": "messages array must not be empty",
                "type": "invalid_request_error",
                "code": "invalid_messages"
            }
        });
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();
    }

    let model =
        request.model.as_deref().filter(|m| !m.is_empty()).unwrap_or(&state.model).to_string();
    let temperature = request.temperature.unwrap_or(state.temperature);
    let stream = request.stream.unwrap_or(false);

    // Convert messages to provider format
    let messages: Vec<ChatMessage> = request
        .messages
        .iter()
        .map(|m| ChatMessage { role: m.role.clone(), content: m.content.clone() })
        .collect();

    let provider_label =
        state.config.lock().default_provider.clone().unwrap_or_else(|| "unknown".to_string());
    let started_at = Instant::now();

    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::LlmRequest {
        provider: provider_label.clone(),
        model: model.clone(),
        messages_count: messages.len(),
    });

    if stream {
        handle_streaming(state, messages, model, temperature, provider_label, started_at)
            .into_response()
    } else {
        handle_non_streaming(state, messages, model, temperature, provider_label, started_at)
            .await
            .into_response()
    }
}

/// Non-streaming chat completions.
async fn handle_non_streaming(
    state: AppState,
    messages: Vec<ChatMessage>,
    model: String,
    temperature: f64,
    provider_label: String,
    started_at: Instant,
) -> impl IntoResponse {
    match state.provider.chat_with_history(&messages, &model, temperature).await {
        Ok(response_text) => {
            let duration = started_at.elapsed();
            record_success(&state, &provider_label, &model, duration);

            #[allow(clippy::cast_possible_truncation)]
            let completion_tokens = (response_text.len() / 4) as u32;
            #[allow(clippy::cast_possible_truncation)]
            let prompt_tokens = messages.iter().map(|m| m.content.len() / 4).sum::<usize>() as u32;

            let response = ChatCompletionsResponse {
                id: format!("chatcmpl-{}", Uuid::new_v4()),
                object: "chat.completion",
                created: unix_timestamp(),
                model: model.clone(),
                choices: vec![ChatCompletionsChoice {
                    index: 0,
                    message: ChatCompletionsResponseMessage {
                        role: "assistant",
                        content: response_text,
                    },
                    finish_reason: "stop",
                }],
                usage: ChatCompletionsUsage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens: prompt_tokens + completion_tokens,
                },
            };

            (StatusCode::OK, Json(serde_json::to_value(response).unwrap())).into_response()
        }
        Err(e) => {
            let duration = started_at.elapsed();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&e.to_string());
            record_failure(&state, &provider_label, &model, duration, &sanitized);

            tracing::error!("/v1/chat/completions provider error: {sanitized}");
            let err = serde_json::json!({
                "error": {
                    "message": "LLM request failed",
                    "type": "server_error",
                    "code": "provider_error"
                }
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
        }
    }
}

/// Streaming chat completions via SSE.
fn handle_streaming(
    state: AppState,
    messages: Vec<ChatMessage>,
    model: String,
    temperature: f64,
    provider_label: String,
    started_at: Instant,
) -> impl IntoResponse {
    let request_id = format!("chatcmpl-{}", Uuid::new_v4());
    let created = unix_timestamp();

    if !state.provider.supports_streaming() {
        // Provider doesn't support streaming — fall back to a single-chunk response
        let model_clone = model.clone();
        let id = request_id.clone();

        let stream = futures_util::stream::once(async move {
            match state.provider.chat_with_history(&messages, &model_clone, temperature).await {
                Ok(text) => {
                    let duration = started_at.elapsed();
                    record_success(&state, &provider_label, &model_clone, duration);

                    Ok::<_, std::io::Error>(chunk_bytes(
                        id.clone(),
                        created,
                        model_clone,
                        Some("assistant"),
                        Some(text),
                        Some("stop"),
                        true,
                    ))
                }
                Err(e) => {
                    let duration = started_at.elapsed();
                    let sanitized =
                        crate::app::agent::providers::sanitize_api_error(&e.to_string());
                    record_failure(&state, &provider_label, &model_clone, duration, &sanitized);

                    Ok(chunk_bytes(
                        id.clone(),
                        created,
                        model_clone,
                        Some("assistant"),
                        Some(format!("LLM request failed: {sanitized}")),
                        Some("stop"),
                        true,
                    ))
                }
            }
        });

        return axum::response::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache")
            .header(header::CONNECTION, "keep-alive")
            .body(Body::from_stream(stream))
            .unwrap()
            .into_response();
    }

    // Provider supports native streaming
    let provider_stream = state.provider.stream_chat_with_history(
        &messages,
        &model,
        temperature,
        StreamOptions::new(true),
    );

    let model_for_stream = model.clone();
    let state_for_stream = state.clone();
    let provider_label_for_stream = provider_label.clone();
    let mut first_chunk = true;
    let mut errored = false;

    let sse_stream = provider_stream.map(move |result| match result {
        Ok(chunk) if chunk.is_final => {
            if !errored {
                let duration = started_at.elapsed();
                record_success(
                    &state_for_stream,
                    &provider_label_for_stream,
                    &model_for_stream,
                    duration,
                );
            }
            Ok::<_, std::io::Error>(axum::body::Bytes::from("data: [DONE]\n\n"))
        }
        Ok(chunk) => {
            let role = if first_chunk {
                first_chunk = false;
                Some("assistant")
            } else {
                None
            };

            Ok(chunk_bytes(
                request_id.clone(),
                created,
                model_for_stream.clone(),
                role,
                if chunk.delta.is_empty() { None } else { Some(chunk.delta) },
                None,
                false,
            ))
        }
        Err(e) => {
            errored = true;
            let duration = started_at.elapsed();
            let msg = e.to_string();
            record_failure(
                &state_for_stream,
                &provider_label_for_stream,
                &model_for_stream,
                duration,
                &msg,
            );
            Ok(chunk_bytes(
                request_id.clone(),
                created,
                model_for_stream.clone(),
                None,
                Some(format!("LLM request failed: {msg}")),
                Some("stop"),
                true,
            ))
        }
    });

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(sse_stream))
        .unwrap()
        .into_response()
}

/// 返回当前网关可解析的 OpenAI 兼容模型列表。
pub async fn handle_v1_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // ── Gateway skey auth ──
    if state.pairing.auth_enabled() {
        let skey = super::api::auth::extract_auth_skey(&headers).unwrap_or("");
        if !state.pairing.is_authenticated(skey) {
            let err = serde_json::json!({
                "error": {
                    "message": "Invalid API key. Send a valid skey as Authorization: Bearer <skey>.",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            });
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    let response = ModelsResponse {
        object: "list",
        data: vec![ModelObject {
            id: state.model.clone(),
            object: "model",
            created: unix_timestamp(),
            owned_by: "vibewindow".to_string(),
        }],
    };

    (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
}

// ══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ══════════════════════════════════════════════════════════════════════════════

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn record_success(
    state: &AppState,
    provider_label: &str,
    model: &str,
    duration: std::time::Duration,
) {
    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::LlmResponse {
        provider: provider_label.to_string(),
        model: model.to_string(),
        duration,
        success: true,
        error_message: None,
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });
    state.observer.record_metric(
        &crate::app::agent::observability::traits::ObserverMetric::RequestLatency(duration),
    );
}

fn record_failure(
    state: &AppState,
    provider_label: &str,
    model: &str,
    duration: std::time::Duration,
    error_message: &str,
) {
    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::LlmResponse {
        provider: provider_label.to_string(),
        model: model.to_string(),
        duration,
        success: false,
        error_message: Some(error_message.to_string()),
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        reasoning_tokens: None,
    });
    state.observer.record_metric(
        &crate::app::agent::observability::traits::ObserverMetric::RequestLatency(duration),
    );
    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::Error {
        component: "gateway".to_string(),
        message: error_message.to_string(),
    });
}

// ══════════════════════════════════════════════════════════════════════════════
// TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
