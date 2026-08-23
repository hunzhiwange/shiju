//! 基于 Axum 的 HTTP 网关，统一路由、认证、限流、请求上限与超时边界。
//!
//! Axum-based HTTP gateway with proper HTTP/1.1 compliance, body limits, and timeouts.
//!
//! This module replaces the raw TCP implementation with axum for:
//! - Proper HTTP/1.1 parsing and compliance
//! - Content-Length validation (handled by hyper)
//! - Request body size limits (32MiB max; covers multimodal / knowledge base64 payloads)
//! - Request timeouts (30s) to prevent slow-loris attacks
//! - Header sanitization (handled by axum/hyper)

mod agent;
pub mod api;
mod approval_state;
mod bootstrap;
mod chat;
mod error;
mod handlers;
mod health;
mod instance;
mod limits;
mod middleware;
#[cfg(test)]
mod mod_tests;
mod node_control;
mod openai_compat;
mod options;
#[cfg(test)]
mod pairing;
mod router;
mod runtime;
pub(crate) mod settings_store;
#[cfg(test)]
mod settings_store_tests;
pub mod sse;
mod state;
#[cfg(test)]
mod tests;
mod types;
mod util;
mod webhook;
mod webhook_ingress;
pub mod ws;

pub use bootstrap::run_gateway;
pub use error::ApiError;
pub use handlers::{WatiVerifyQuery, WhatsAppVerifyQuery, verify_whatsapp_signature};
pub use limits::{
    GatewayRateLimiter, IDEMPOTENCY_MAX_KEYS_DEFAULT, IdempotencyStore,
    RATE_LIMIT_MAX_KEYS_DEFAULT, RATE_LIMIT_WINDOW_SECS, RATE_LIMITER_SWEEP_INTERVAL_SECS,
    SlidingWindowRateLimiter,
};
pub use node_control::NodeControlRequest;
pub use options::ServeOptions;
pub use runtime::serve;
pub use runtime::start;
pub use state::AppState;
pub use types::{AgentBody, WebhookBody};
pub use util::{
    client_key_from_request, forwarded_client_ip, hash_webhook_secret, linq_memory_key,
    nextcloud_talk_memory_key, normalize_max_keys, parse_client_ip, qq_memory_key, wati_memory_key,
    webhook_memory_key, whatsapp_memory_key,
};

use axum::{
    body::Body,
    extract::{Request, State},
    http::Method,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Duration;

/// Maximum request body size (32MiB).
///
/// Sized to fit multimodal / knowledge uploads after base64 expansion:
/// desktop and knowledge paths allow up to 20MiB raw files (~27MiB base64),
/// with headroom for JSON wrappers and short chat history.
pub const MAX_BODY_SIZE: usize = 32 * 1024 * 1024;
/// Request timeout (30s) — prevents slow-loris attacks
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Long-running SSE timeout (1h) — chat / workflow / task-plan-run streams.
pub const WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS: u64 = 3_600;

async fn request_timeout_middleware(req: Request<Body>, next: Next) -> Response {
    let timeout = request_timeout_for_path(req.uri().path());
    match tokio::time::timeout(timeout, next.run(req)).await {
        Ok(response) => response,
        Err(_) => StatusCode::REQUEST_TIMEOUT.into_response(),
    }
}

fn request_timeout_for_path(path: &str) -> Duration {
    if is_long_running_sse_path(path) {
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    } else {
        Duration::from_secs(REQUEST_TIMEOUT_SECS)
    }
}

fn is_long_running_sse_path(path: &str) -> bool {
    is_workflow_chat_messages_path(path)
        || path == "/v1/chat/stream"
        || path == "/v1/task-plan-run/stream"
}

fn is_workflow_chat_messages_path(path: &str) -> bool {
    path == "/v1/workflow/applications/chat-messages"
        || (path.starts_with("/v1/workflow/applications/")
            && path.ends_with("/chat-messages")
            && path
                .trim_end_matches("/chat-messages")
                .trim_start_matches("/v1/workflow/applications/")
                .contains(|ch: char| !ch.is_whitespace()))
}

async fn skey_auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS || is_public_skey_auth_path(req.uri().path()) {
        return next.run(req).await;
    }

    if let Err(err) = api::auth::require_auth(&state, req.headers()) {
        return err.into_response();
    }

    next.run(req).await
}

fn is_public_skey_auth_path(path: &str) -> bool {
    matches!(path, "/v1/global/health" | "/global/health")
}
