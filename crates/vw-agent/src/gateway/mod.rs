//! 基于 Axum 的 HTTP 网关，统一路由、认证、限流、请求上限与超时边界。
//!
//! 模块只导出稳定启动入口、共享状态及必要协议类型，其余路由装配保持在内部模块。
//!
//! Axum-based HTTP gateway with proper HTTP/1.1 compliance, body limits, and timeouts.
//!
//! This module replaces the raw TCP implementation with axum for:
//! - Proper HTTP/1.1 parsing and compliance
//! - Content-Length validation (handled by hyper)
//! - Webhook request body size limits (32MiB); client payload sizes are validated by providers
//! - Request timeouts (30s) to prevent slow-loris attacks
//! - Header sanitization (handled by axum/hyper)

mod agent;
pub(crate) mod agent_apps;
pub mod agent_apps_diagnostic_executors;
pub mod agent_apps_diagnostic_jobs;
#[cfg(target_os = "macos")]
pub(crate) mod agent_apps_failover_runtime;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_failover_runtime_tests;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_failover_timeouts_tests;
#[cfg(target_os = "macos")]
pub mod agent_apps_high_availability_service;
#[cfg(all(test, target_os = "macos"))]
mod agent_apps_high_availability_service_tests;
pub mod agent_apps_high_availability_store;
pub mod agent_apps_skills_store;
pub mod api;
pub(crate) mod approval_state;
mod bootstrap;
mod channel_reload;
mod channel_runtime;
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
#[cfg(test)]
pub(crate) use bootstrap::dynamic_tools_acceptance_router;
mod runtime;
pub(crate) mod settings_store;
pub mod sse;
mod state;
#[cfg(test)]
mod tests;
mod types;
mod util;
mod webhook;
#[cfg(test)]
mod webhook_ingress;
pub mod ws;

pub(crate) use bootstrap::run_gateway_until;
pub use bootstrap::{GatewayRouteEntry, documented_gateway_routes, run_gateway};
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

pub use agent_gateway::request_policy::{
    MAX_BODY_SIZE, REQUEST_TIMEOUT_SECS, WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS,
};
use agent_gateway::request_policy::{request_timeout_middleware, skey_auth_middleware};
#[cfg(test)]
use agent_gateway::request_policy::{request_timeout_for_path, is_public_skey_auth_path, is_workflow_chat_messages_path};

#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/recovery_window_tests.rs"]
mod recovery_window_tests;

#[cfg(target_os = "macos")]
pub(crate) mod aggregation_resync;

#[cfg(target_os = "macos")]
pub(crate) mod aggregation_desktop_record;

#[cfg(target_os = "macos")]
pub(crate) mod aggregation_lifecycle;

// Exercise production route registration from cross-module acceptance tests.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn wecom_acceptance_router(
    state: AppState,
    agent: axum::Router<AppState>,
    assets: axum::Router<AppState>,
) -> axum::Router<AppState> {
    bootstrap::client_router(state, agent, assets)
}

#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/deepagents_window_tests.rs"]
mod deepagents_window_tests;
