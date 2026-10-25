//! Webhook 入口请求处理模块
//!
//! 本模块负责处理来自外部系统的 Webhook 请求，提供完整的请求处理流水线，
//! 包括认证、限流、幂等性检查、消息持久化以及 LLM 调用等功能。
//!
//! # 核心功能
//!
//! - **请求处理**: 解析和验证 Webhook 请求，执行 LLM 推理并返回响应
//! - **安全控制**: 支持 gateway skey、Webhook 密钥验证以及速率限制
//! - **幂等性保证**: 通过幂等键防止重复请求处理
//! - **可观测性**: 记录遥测数据，包括请求延迟、成功/失败状态等指标
//!
//! # 认证机制
//!
//! 模块支持三种认证方式（按优先级）：
//! 1. **Gateway 鉴权**: 通过 `Authorization: Bearer <skey>` 发送 skey
//! 2. **Webhook 密钥**: 通过 `X-Webhook-Secret` 头传递预共享密钥
//! 3. **本地回环地址**: 允许来自 `127.0.0.1` 或 `::1` 的请求（仅当其他认证方式未配置时）
//!
//! # 幂等性支持
//!
//! 客户端可通过 `X-Idempotency-Key` 请求头传递幂等键，系统会自动检测并拒绝
//! 重复的请求，返回 `200 OK` 状态码和 `duplicate` 标识。

use super::chat::{run_gateway_chat_simple, sanitize_gateway_response};
use super::util::webhook_memory_key;
use super::{
    AppState, RATE_LIMIT_WINDOW_SECS, WebhookBody, client_key_from_request, hash_webhook_secret,
};
use crate::app::agent::memory::MemoryCategory;
use crate::app::agent::providers;
use crate::app::agent::security::pairing::constant_time_eq;
use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::{HeaderMap, StatusCode},
};
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Instant;

/// Webhook 请求的遥测数据收集器
///
/// 用于追踪单个 Webhook 请求的生命周期，记录从请求开始到结束的各类指标。
/// 支持成功和失败两种结束状态的指标记录。
///
/// # 字段说明
///
/// - `provider_label`: 使用的 LLM 提供商标识（如 "openai"、"anthropic"）
/// - `model_label`: 使用的模型标识（如 "gpt-4"、"claude-3-opus"）
/// - `started_at`: 请求开始的时间戳，用于计算总耗时
struct WebhookTelemetry {
    provider_label: String,
    model_label: String,
    started_at: Instant,
}

impl WebhookTelemetry {
    /// 启动新的遥测追踪会话
    ///
    /// 从应用状态中提取提供者和模型信息，记录 Agent 启动和 LLM 请求事件。
    /// 返回一个新的 `WebhookTelemetry` 实例用于后续的状态追踪。
    ///
    /// # 参数
    ///
    /// - `state`: 应用共享状态的引用，从中读取配置的提供者和模型信息
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `WebhookTelemetry` 实例，已记录启动事件
    fn start(state: &AppState) -> Self {
        // 从配置中获取默认提供者，若未配置则使用 "unknown"
        let provider_label =
            state.config.lock().default_provider.clone().unwrap_or_else(|| "unknown".to_string());
        let model_label = state.model.clone();
        let started_at = Instant::now();

        // 记录 Agent 启动事件
        state.observer.record_event(&crate::app::agent::observability::ObserverEvent::AgentStart {
            provider: provider_label.clone(),
            model: model_label.clone(),
        });
        // 记录 LLM 请求事件，消息数量固定为 1（单条消息）
        state.observer.record_event(&crate::app::agent::observability::ObserverEvent::LlmRequest {
            provider: provider_label.clone(),
            model: model_label.clone(),
            messages_count: 1,
        });

        Self { provider_label, model_label, started_at }
    }

    /// 完成遥测追踪（成功状态）
    ///
    /// 当请求成功完成时调用，记录成功响应、请求延迟和 Agent 结束事件。
    /// 消耗 `self`，表示遥测会话结束。
    ///
    /// # 参数
    ///
    /// - `state`: 应用共享状态的引用，用于记录观测事件和指标
    fn finish_success(self, state: &AppState) {
        let duration = self.started_at.elapsed();

        // 记录 LLM 响应事件（成功状态）
        state.observer.record_event(
            &crate::app::agent::observability::ObserverEvent::LlmResponse {
                provider: self.provider_label.clone(),
                model: self.model_label.clone(),
                duration,
                success: true,
                error_message: None,
                input_tokens: None,
                output_tokens: None,
                cached_tokens: None,
                reasoning_tokens: None,
            },
        );
        // 记录请求延迟指标
        state.observer.record_metric(
            &crate::app::agent::observability::traits::ObserverMetric::RequestLatency(duration),
        );
        // 记录 Agent 结束事件
        state.observer.record_event(&crate::app::agent::observability::ObserverEvent::AgentEnd {
            provider: self.provider_label,
            model: self.model_label,
            duration,
            tokens_used: None,
            cost_usd: None,
        });
    }

    /// 完成遥测追踪（错误状态）
    ///
    /// 当请求失败时调用，记录错误响应、请求延迟、错误事件和 Agent 结束事件。
    /// 消耗 `self`，表示遥测会话结束。
    ///
    /// # 参数
    ///
    /// - `state`: 应用共享状态的引用，用于记录观测事件和指标
    /// - `error_message`: 错误消息内容，将被记录到错误事件中
    fn finish_error(self, state: &AppState, error_message: &str) {
        let duration = self.started_at.elapsed();
        let sanitized = error_message.to_string();

        // 记录 LLM 响应事件（失败状态）
        state.observer.record_event(
            &crate::app::agent::observability::ObserverEvent::LlmResponse {
                provider: self.provider_label.clone(),
                model: self.model_label.clone(),
                duration,
                success: false,
                error_message: Some(sanitized.clone()),
                input_tokens: None,
                output_tokens: None,
                cached_tokens: None,
                reasoning_tokens: None,
            },
        );
        // 记录请求延迟指标
        state.observer.record_metric(
            &crate::app::agent::observability::traits::ObserverMetric::RequestLatency(duration),
        );
        // 记录组件错误事件
        state.observer.record_event(&crate::app::agent::observability::ObserverEvent::Error {
            component: "gateway".to_string(),
            message: sanitized.clone(),
        });
        // 记录 Agent 结束事件
        state.observer.record_event(&crate::app::agent::observability::ObserverEvent::AgentEnd {
            provider: self.provider_label,
            model: self.model_label,
            duration,
            tokens_used: None,
            cost_usd: None,
        });
    }
}

/// Webhook 请求的内部处理函数
///
/// 这是 Webhook 请求处理的核心函数，执行完整的请求处理流水线。
/// 处理流程包括：限流检查 → 认证授权 → 请求体解析 → 幂等性检查 → 消息持久化 → LLM 调用 → 响应返回。
///
/// # 参数
///
/// - `state`: 应用共享状态，包含配置、内存、限流器、认证器等组件
/// - `peer_addr`: 客户端的套接字地址（IP 和端口）
/// - `headers`: HTTP 请求头的映射集合
/// - `body`: 解析后的 JSON 请求体，或解析失败的错误信息
///
/// # 返回值
///
/// 返回元组 `(StatusCode, Json<Value>)`，其中：
/// - `StatusCode`: HTTP 状态码（200/400/401/429/500 等）
/// - `Json<Value>`: JSON 格式的响应体
///
/// # 处理流程
///
/// 1. **限流检查**: 基于 IP 或认证标识限制请求频率
/// 2. **认证授权**: 验证 gateway skey 或 Webhook 密钥
/// 3. **请求体解析**: 解析 JSON 格式的请求体
/// 4. **幂等性检查**: 如果提供幂等键且已处理过，直接返回重复响应
/// 5. **消息持久化**: 根据配置保存入站消息到记忆系统
/// 6. **LLM 调用**: 执行语言模型推理
/// 7. **响应返回**: 返回推理结果或错误信息
pub(super) async fn handle_webhook_inner(
    state: AppState,
    peer_addr: SocketAddr,
    headers: HeaderMap,
    body: Result<Json<WebhookBody>, JsonRejection>,
) -> (StatusCode, Json<Value>) {
    // 步骤 1: 速率限制检查，防止滥用
    if let Some(response) = enforce_rate_limit(&state, peer_addr, &headers) {
        return response;
    }

    // 步骤 2: 认证和授权检查
    if let Some(response) = authorize_webhook_request(&state, peer_addr, &headers) {
        return response;
    }

    // 步骤 3: 解析请求体
    let webhook_body = match parse_webhook_body(body) {
        Ok(webhook_body) => webhook_body,
        Err(response) => return response,
    };

    // 步骤 4: 幂等性检查（如果提供了幂等键）
    if let Some(response) = enforce_idempotency(&state, &headers) {
        return response;
    }

    // 步骤 5: 可选地持久化入站消息到记忆系统
    maybe_persist_inbound_message(&state, &webhook_body.message).await;

    // 启动遥测追踪
    let telemetry = WebhookTelemetry::start(&state);

    // 步骤 6: 调用 LLM 进行推理
    match run_gateway_chat_simple(&state, &webhook_body.message).await {
        Ok(response) => {
            // 清理响应内容，防止敏感信息泄露
            let safe_response =
                sanitize_gateway_response(&response, state.tools_registry_exec.as_ref());
            telemetry.finish_success(&state);
            let body = serde_json::json!({"response": safe_response, "model": state.model});
            (StatusCode::OK, Json(body))
        }
        Err(error) => {
            // 清理错误消息，移除敏感信息
            let sanitized = providers::sanitize_api_error(&error.to_string());
            telemetry.finish_error(&state, &sanitized);
            tracing::error!("Webhook provider error: {}", sanitized);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "LLM request failed"})),
            )
        }
    }
}

/// 执行请求速率限制检查
///
/// 基于客户端标识（IP 或转发头）限制 Webhook 请求的频率，防止滥用。
///
/// # 参数
///
/// - `state`: 应用共享状态的引用
/// - `peer_addr`: 客户端的套接字地址
/// - `headers`: HTTP 请求头的引用
///
/// # 返回值
///
/// - `None`: 请求未超过速率限制，可以继续处理
/// - `Some((StatusCode, Json<Value>))`: 请求被限流，返回 429 状态码和重试信息
fn enforce_rate_limit(
    state: &AppState,
    peer_addr: SocketAddr,
    headers: &HeaderMap,
) -> Option<(StatusCode, Json<Value>)> {
    // 生成客户端唯一标识（优先使用转发头中的信息，否则使用 IP）
    let rate_key = client_key_from_request(Some(peer_addr), headers, state.trust_forwarded_headers);
    if state.rate_limiter.allow_webhook(&rate_key) {
        return None;
    }

    // 速率限制触发，记录警告并返回 429 响应
    tracing::warn!("/webhook rate limit exceeded");
    Some((
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        })),
    ))
}

/// 执行 Webhook 请求的认证和授权检查
///
/// 按照以下优先级顺序验证请求的身份：
/// 1. 检查是否需要认证（配对启用、Webhook 密钥配置、非本地请求）
/// 2. 如果启用了 gateway skey 鉴权，验证 Authorization Bearer
/// 3. 如果配置了 Webhook 密钥，验证 X-Webhook-Secret 头
///
/// # 安全策略
///
/// - **本地请求**: 当配对未启用且无 Webhook 密钥时，允许来自回环地址的请求
/// - **远程请求**: 必须提供有效的 skey Bearer 或 Webhook 密钥
///
/// # 参数
///
/// - `state`: 应用共享状态的引用
/// - `peer_addr`: 客户端的套接字地址
/// - `headers`: HTTP 请求头的引用
///
/// # 返回值
///
/// - `None`: 认证通过或无需认证，可以继续处理
/// - `Some((StatusCode, Json<Value>))`: 认证失败，返回 401 状态码和错误信息
fn authorize_webhook_request(
    state: &AppState,
    peer_addr: SocketAddr,
    headers: &HeaderMap,
) -> Option<(StatusCode, Json<Value>)> {
    let channels = state.channels.lock().clone();
    // 检查是否需要认证：非本地请求且未配置任何认证机制时拒绝
    if !state.pairing.auth_enabled()
        && channels.webhook_secret_hash.is_none()
        && !peer_addr.ip().is_loopback()
    {
        tracing::warn!(
            "Webhook: rejected unauthenticated non-loopback request (gateway auth disabled and no webhook secret configured)"
        );
        return Some((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — configure gateway auth or X-Webhook-Secret for non-local webhook access"
            })),
        ));
    }

    // 如果启用了 gateway skey 鉴权，验证 Authorization Bearer
    if state.pairing.auth_enabled() {
        let skey = super::api::auth::extract_auth_skey(headers).unwrap_or("");
        if !state.pairing.is_authenticated(skey) {
            tracing::warn!("Webhook: rejected — invalid gateway skey");
            return Some((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized — send a valid skey as Authorization: Bearer <skey>"
                })),
            ));
        }
    }

    // 如果配置了 Webhook 密钥，验证 X-Webhook-Secret 头（使用常量时间比较防止时序攻击）
    if let Some(ref secret_hash) = channels.webhook_secret_hash {
        match extract_webhook_secret_header_hash(headers) {
            Some(header_hash) if constant_time_eq(&header_hash, secret_hash.as_ref()) => {}
            _ => {
                tracing::warn!("Webhook: rejected request — invalid or missing X-Webhook-Secret");
                return Some((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "Unauthorized — invalid or missing X-Webhook-Secret header"
                    })),
                ));
            }
        }
    }

    None
}

/// 解析 Webhook 请求的 JSON 请求体
///
/// 将 Axum 框架提供的 JSON 解析结果转换为内部使用的 `WebhookBody` 结构。
/// 如果解析失败，返回 400 Bad Request 响应。
///
/// # 参数
///
/// - `body`: JSON 解析结果，可能成功包含 `WebhookBody` 或包含解析错误
///
/// # 返回值
///
/// - `Ok(WebhookBody)`: 成功解析的请求体
/// - `Err((StatusCode, Json<Value>))`: 解析失败，返回 400 状态码和错误信息
fn parse_webhook_body(
    body: Result<Json<WebhookBody>, JsonRejection>,
) -> Result<WebhookBody, (StatusCode, Json<Value>)> {
    match body {
        Ok(Json(webhook_body)) => Ok(webhook_body),
        Err(error) => {
            tracing::warn!("Webhook JSON parse error: {error}");
            Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid JSON body. Expected: {\"message\": \"...\"}"
                })),
            ))
        }
    }
}

/// 执行幂等性检查
///
/// 如果请求包含 `X-Idempotency-Key` 头，检查该键是否已被处理过。
/// 如果已处理过，返回幂等响应（200 OK + duplicate 标识），避免重复执行。
///
/// # 参数
///
/// - `state`: 应用共享状态的引用，包含幂等性存储
/// - `headers`: HTTP 请求头的引用
///
/// # 返回值
///
/// - `None`: 无幂等键或首次见到该键，可以继续处理
/// - `Some((StatusCode, Json<Value>))`: 检测到重复请求，返回 200 状态码和重复标识
fn enforce_idempotency(state: &AppState, headers: &HeaderMap) -> Option<(StatusCode, Json<Value>)> {
    // 提取幂等键，如果不存在则跳过幂等性检查
    let idempotency_key = extract_idempotency_key(headers)?;

    // 尝试记录该幂等键，如果是首次见到则返回 true
    if state.idempotency_store.record_if_new(idempotency_key) {
        return None;
    }

    // 幂等键已存在，返回重复响应
    tracing::info!("Webhook duplicate ignored (idempotency key: {idempotency_key})");
    Some((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "duplicate",
            "idempotent": true,
            "message": "Request already processed for this idempotency key"
        })),
    ))
}

/// 从请求头中提取幂等键
///
/// 从 `X-Idempotency-Key` 头中提取并清理幂等键字符串。
/// 如果头不存在、格式无效或为空字符串，返回 `None`。
///
/// # 参数
///
/// - `headers`: HTTP 请求头的引用
///
/// # 返回值
///
/// - `Some(&str)`: 有效且非空的幂等键字符串
/// - `None`: 幂等键不存在或无效
fn extract_idempotency_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("X-Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 从请求头中提取并哈希 Webhook 密钥
///
/// 从 `X-Webhook-Secret` 头中提取密钥字符串，对其进行哈希处理后返回。
/// 哈希处理确保在比较时可以使用常量时间算法，防止时序攻击。
///
/// # 参数
///
/// - `headers`: HTTP 请求头的引用
///
/// # 返回值
///
/// - `Some(String)`: 有效且非空的 Webhook 密钥的哈希值
/// - `None`: 密钥头不存在或无效
fn extract_webhook_secret_header_hash(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Webhook-Secret")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(hash_webhook_secret)
}

/// 可选地持久化入站消息到记忆系统
///
/// 根据应用配置决定是否将入站的 Webhook 消息保存到记忆存储中。
/// 仅当 `auto_save` 配置项为 `true` 时执行持久化。
///
/// # 参数
///
/// - `state`: 应用共享状态的引用
/// - `message`: 要保存的消息内容
///
/// # 错误处理
///
/// 持久化失败时错误会被静默忽略（使用 `let _ = ...`），避免影响主流程。
async fn maybe_persist_inbound_message(state: &AppState, message: &str) {
    // 检查是否启用了自动保存功能
    if !state.auto_save {
        return;
    }

    // 生成记忆键并保存消息到对话类别
    let key = webhook_memory_key();
    let _ = state.mem.store(&key, message, MemoryCategory::Conversation, None).await;
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
