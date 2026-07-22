//! Agent HTTP 端点模块
//!
//! 本模块提供基于 HTTP POST 的单轮 Agent 交互端点，支持工具执行。
//! 这是一个兼容性路由，为期望 JSON POST API 而非 WebSocket 聊天的调用者
//! 提供类似 CLI 风格的 Agent 行为。
//!
//! # 主要功能
//!
//! - 提供认证的单轮 Agent 端点
//! - 支持速率限制以防止滥用
//! - 支持配对认证机制
//! - 集成工具执行能力
//! - 自动保存对话记忆
//! - 完整的观测事件记录

use super::RATE_LIMIT_WINDOW_SECS;
use super::chat::{run_gateway_chat_with_tools, sanitize_gateway_response};
use super::state::AppState;
use super::types::AgentBody;
use super::util::webhook_memory_key;
use crate::app::agent::memory::MemoryCategory;
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use std::net::SocketAddr;
use std::time::Instant;

/// 处理 Agent HTTP POST 请求
///
/// 这是一个认证的单轮 Agent 端点，支持工具执行。该路由为期望 JSON POST API
/// 而非 WebSocket 聊天的调用者提供类似 CLI 风格的 Agent 行为。
///
/// # 端点
///
/// `POST /agent`
///
/// # 参数
///
/// - `state`: 应用状态，包含配置、内存、速率限制器等共享资源
/// - `peer_addr`: 客户端的套接字地址，用于速率限制和日志记录
/// - `headers`: HTTP 请求头，用于提取认证令牌和客户端标识
/// - `body`: 请求体，包含 Agent 消息内容
///
/// # 返回值
///
/// 返回 HTTP 响应，可能包含以下状态码：
/// - `200 OK`: 成功处理请求，返回 Agent 响应
/// - `400 BAD_REQUEST`: 请求体格式错误或消息为空
/// - `401 UNAUTHORIZED`: 未认证，需要先完成配对
/// - `429 TOO_MANY_REQUESTS`: 超过速率限制
/// - `502 BAD_GATEWAY`: 提供者（Provider）处理错误
///
/// # 认证机制
///
/// 如果配对机制启用，请求必须包含有效的 Bearer 令牌：
/// ```http
/// Authorization: Bearer <token>
/// ```
///
/// # 请求体格式
///
/// ```json
/// {
///     "message": "你的问题或指令"
/// }
/// ```
///
/// # 成功响应格式
///
/// ```json
/// {
///     "response": "Agent 的回复内容"
/// }
/// ```
///
/// # 示例
///
/// ```bash
/// curl -X POST http://localhost:8080/agent \
///   -H "Content-Type: application/json" \
///   -H "Authorization: Bearer your-token" \
///   -d '{"message": "你好，请帮我分析这段代码"}'
/// ```
///
/// # 观测事件
///
/// 该函数会记录以下观测事件：
/// - `AgentStart`: Agent 开始处理
/// - `LlmRequest`: LLM 请求开始
/// - `LlmResponse`: LLM 响应（成功或失败）
/// - `TurnComplete`: 单轮交互完成
/// - `AgentEnd`: Agent 处理结束
pub async fn handle_agent(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<AgentBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    // 从请求中提取客户端标识键（用于速率限制）
    // 优先使用转发头中的客户端信息（如果信任转发头）
    let rate_key = super::util::client_key_from_request(
        Some(peer_addr),
        &headers,
        state.trust_forwarded_headers,
    );

    // 检查速率限制，如果超过限制则返回 429 错误
    if !state.rate_limiter.allow_webhook(&rate_key) {
        tracing::warn!("/agent rate limit exceeded");
        let err = serde_json::json!({
            "error": "Too many agent requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err)).into_response();
    }

    if let Err(err) = super::api::auth::require_auth(&state, &headers) {
        return err.into_response();
    }

    // 解析请求体，如果 JSON 格式错误则返回 400 错误
    let Json(agent_body) = match body {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("/agent JSON parse error: {e}");
            let err = serde_json::json!({
                "error": "Invalid JSON body. Expected: {\"message\": \"...\"}"
            });
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();
        }
    };

    // 验证消息内容不为空
    let message = agent_body.message.trim();
    if message.is_empty() {
        let err = serde_json::json!({
            "error": "message must not be empty"
        });
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();
    }

    // 如果启用了自动保存，将用户消息存储到内存中
    let key = webhook_memory_key();
    if state.auto_save {
        let _ = state.mem.store(&key, message, MemoryCategory::Conversation, None).await;
    }

    // 获取当前配置的提供者和模型标签，用于观测记录
    let provider_label =
        state.config.lock().default_provider.clone().unwrap_or_else(|| "unknown".to_string());
    let model_label = state.model.clone();
    let started_at = Instant::now();

    // 记录 Agent 启动事件和 LLM 请求事件
    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::AgentStart {
        provider: provider_label.clone(),
        model: model_label.clone(),
    });
    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::LlmRequest {
        provider: provider_label.clone(),
        model: model_label.clone(),
        messages_count: 1,
    });

    // 执行带有工具支持的聊天交互
    let response = match run_agent_chat(&state, message, &key).await {
        Ok(response) => {
            // 对响应进行安全清理，移除敏感信息
            let safe = sanitize_gateway_response(&response, state.tools_registry_exec.as_ref());

            // 记录 LLM 成功响应事件
            state.observer.record_event(
                &crate::app::agent::observability::ObserverEvent::LlmResponse {
                    provider: provider_label.clone(),
                    model: model_label.clone(),
                    duration: started_at.elapsed(),
                    success: true,
                    error_message: None,
                    input_tokens: None,
                    output_tokens: None,
                    cached_tokens: None,
                    reasoning_tokens: None,
                },
            );

            // 记录单轮交互完成事件
            state
                .observer
                .record_event(&crate::app::agent::observability::ObserverEvent::TurnComplete);
            safe
        }
        Err(e) => {
            // 清理错误消息，避免泄露敏感信息
            let sanitized = crate::app::agent::providers::sanitize_api_error(&e.to_string());

            // 记录 LLM 失败响应事件
            state.observer.record_event(
                &crate::app::agent::observability::ObserverEvent::LlmResponse {
                    provider: provider_label.clone(),
                    model: model_label.clone(),
                    duration: started_at.elapsed(),
                    success: false,
                    error_message: Some(sanitized.clone()),
                    input_tokens: None,
                    output_tokens: None,
                    cached_tokens: None,
                    reasoning_tokens: None,
                },
            );

            // 返回 502 错误，表示上游提供者处理失败
            let err = serde_json::json!({
                "error": format!("Provider error: {sanitized}")
            });
            return (StatusCode::BAD_GATEWAY, Json(err)).into_response();
        }
    };

    // 记录 Agent 结束事件，包含完整的执行统计信息
    state.observer.record_event(&crate::app::agent::observability::ObserverEvent::AgentEnd {
        provider: provider_label,
        model: model_label,
        duration: started_at.elapsed(),
        tokens_used: None,
        cost_usd: None,
    });

    // 返回成功的 JSON 响应
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "response": response
        })),
    )
        .into_response()
}

#[cfg(not(test))]
async fn run_agent_chat(state: &AppState, message: &str, key: &str) -> anyhow::Result<String> {
    run_gateway_chat_with_tools(state, message, key).await
}

#[cfg(test)]
async fn run_agent_chat(state: &AppState, message: &str, key: &str) -> anyhow::Result<String> {
    if state.config.lock().default_provider.as_deref() == Some("static") {
        return super::chat::run_gateway_chat_simple(state, message).await;
    }
    run_gateway_chat_with_tools(state, message, key).await
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;
