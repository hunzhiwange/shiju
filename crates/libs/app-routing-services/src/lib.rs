//! Bounded HTTP entity processing and admission for application routes.
pub mod body_limits;
pub mod browser_guard;
pub mod cancellation;
pub mod capacity;
pub mod decode_request;
pub mod decode_response;
pub mod reference_scope;
#[cfg(target_os = "macos")]
pub mod config_apply;
#[cfg(target_os = "macos")]
pub mod desktop_routing;
#[cfg(target_os = "macos")]
pub mod adapter_selection;
pub mod protocol_errors;
pub mod timeouts;
#[cfg(target_os = "macos")]
pub mod failover_runtime;
#[cfg(target_os = "macos")]
pub mod failover_store;
#[cfg(target_os = "macos")]
pub mod high_availability_service;
#[cfg(target_os = "macos")]
pub mod lifecycle_host;
#[cfg(target_os = "macos")]
pub mod aggregation_routing;
pub mod listener;
pub mod route_auth;
pub mod dispatch;
pub mod shared_transport;
#[cfg(target_os = "macos")]
pub mod outbound_config;
pub mod outbound_custom;
#[cfg(target_os = "macos")]
pub mod outbound_system;
#[cfg(target_os = "macos")]
pub mod provider_publication;
#[cfg(target_os = "macos")]
pub mod failover_routing;
#[cfg(target_os = "macos")]
pub mod failover_execution;
pub mod header_wire;
pub mod override_apply;
pub mod shutdown;
pub mod transport_security;
pub mod request_status;
pub use listener::{ListenerError, RoutingListener};
pub use shared_transport::SharedTransport;
#[cfg(target_os = "macos")]
pub use route_auth::{RouteCommitError, RouteRevocation, VerifiedRouteBinding};

#[cfg(all(test, target_os = "macos"))]
mod body_limits_tests;

#[cfg(all(test, target_os = "macos"))]
mod config_apply_tests;

#[cfg(test)]
mod listener_tests;

#[cfg(test)]
mod shared_transport_tests;

#[cfg(all(test, target_os = "macos"))]
mod outbound_config_tests;

#[cfg(all(test, target_os = "macos"))]
mod outbound_custom_tests;

#[cfg(all(test, target_os = "macos"))]
mod outbound_system_tests;

#[cfg(test)]
mod reference_scope_tests;

#[cfg(test)]
mod shutdown_tests;

#[cfg(all(test, target_os = "macos"))]
mod timeouts_tests;

#[cfg(all(test, target_os = "macos"))]
mod cancellation_tests;

#[cfg(all(test, target_os = "macos"))]
mod capacity_tests;

#[cfg(all(test, target_os = "macos"))]
mod protocol_errors_tests;

#[cfg(all(test, target_os = "macos"))]
mod transport_security_tests;

#[cfg(all(test, target_os = "macos"))]
mod compaction_tests;

#[cfg(test)]
use crate as app_routing;
#[cfg(test)]
fn test_home() -> std::path::PathBuf {
    static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    HOME.get_or_init(|| tempfile::tempdir().unwrap()).path().to_owned()
}
#[cfg(test)]
fn test_database() -> agent_database::GlobalDatabase {
    agent_database::GlobalDatabase::for_home(&test_home()).unwrap()
}
#[cfg(test)]
fn test_listener() -> RoutingListener {
    RoutingListener::with_database(test_database())
}

#[cfg(test)]
fn test_samples() -> Vec<(&'static str, &'static str, String)> {
    let data = |value: serde_json::Value| format!("data: {value}\n\n");
    vec![
        ("openai_chat", "/v1/chat/completions", [
            data(serde_json::json!({"object":"chat.completion.chunk","model":"fixture-v1","choices":[{"index":0,"delta":{"role":"assistant","content":"OK"},"finish_reason":null}]})),
            data(serde_json::json!({"object":"chat.completion.chunk","model":"fixture-v1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})),
            "data: [DONE]\n\n".into(),
        ].concat()),
        ("anthropic", "/v1/messages", [
            data(serde_json::json!({"type":"message_start","message":{"id":"m","role":"assistant","model":"fixture-v1","type":"message","content":[]}})),
            data(serde_json::json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})),
            data(serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"OK"}})),
            data(serde_json::json!({"type":"content_block_stop","index":0})),
            data(serde_json::json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}})),
            data(serde_json::json!({"type":"message_stop"})),
        ].concat()),
        ("openai_responses", "/v1/responses", [
            data(serde_json::json!({"type":"response.created","response":{"id":"r","object":"response","model":"fixture-v1","status":"in_progress"}})),
            data(serde_json::json!({"type":"response.output_text.delta","delta":"OK","item_id":"m","output_index":0,"content_index":0})),
            data(serde_json::json!({"type":"response.completed","response":{"id":"r","object":"response","model":"fixture-v1","status":"completed"}})),
        ].concat()),
        ("gemini_native", "/v1/models/fixture:streamGenerateContent?alt=sse", [
            data(serde_json::json!({"modelVersion":"fixture-v1","candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"OK"}]}}]})),
            data(serde_json::json!({"modelVersion":"fixture-v1","candidates":[{"index":0,"finishReason":"STOP"}]})),
        ].concat()),
    ]
}

#[cfg(all(test, target_os = "macos"))]
mod host_migration_13_tests;

#[cfg(all(test, target_os = "macos"))]
mod host_migration_57_tests;

pub mod gateway_owner;
