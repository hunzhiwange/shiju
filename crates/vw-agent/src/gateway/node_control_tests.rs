use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::{Body, to_bytes};
use axum::extract::{FromRequest, State};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, header};
use axum::response::Response;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

#[test]
fn node_control_request_deserializes_method_and_node() {
    let request: NodeControlRequest = serde_json::from_value(serde_json::json!({
        "method": "node.list",
        "node_id": "node-a"
    }))
    .expect("valid request");

    assert_eq!(request.method, "node.list");
    assert_eq!(request.node_id.as_deref(), Some("node-a"));
}

#[test]
fn node_control_request_deserializes_capability_and_arguments() {
    let arguments = serde_json::json!({"command": "status", "timeout_secs": 5});
    let request: NodeControlRequest = serde_json::from_value(serde_json::json!({
        "method": "node.invoke",
        "node_id": "node-a",
        "capability": "shell",
        "arguments": arguments.clone()
    }))
    .expect("valid request");

    assert_eq!(request.capability.as_deref(), Some("shell"));
    assert_eq!(request.arguments, arguments);
}

#[test]
fn node_control_request_deserializes_defaults() {
    let request: NodeControlRequest = serde_json::from_value(serde_json::json!({
        "method": "node.invoke"
    }))
    .expect("valid request");

    assert_eq!(request.method, "node.invoke");
    assert_eq!(request.node_id, None);
    assert_eq!(request.capability, None);
    assert_eq!(request.arguments, serde_json::Value::Null);
}

#[test]
fn node_id_allowed_accepts_empty_allowlist() {
    assert!(node_id_allowed("node-a", &[]));
}

#[test]
fn node_id_allowed_accepts_matching_node_or_wildcard() {
    assert!(node_id_allowed("node-a", &["node-a".to_string()]));
    assert!(node_id_allowed("node-b", &["*".to_string()]));
}

#[test]
fn node_id_allowed_rejects_non_matching_node() {
    assert!(!node_id_allowed("node-a", &["node-b".to_string()]));
}

#[test]
fn node_id_allowed_rejects_partial_wildcard() {
    assert!(!node_id_allowed("node-a", &["node-*".to_string()]));
}

#[tokio::test]
async fn handle_node_control_rejects_missing_pairing_bearer() {
    let response = request(
        state_with(config_with_node_control(true, &[], None), true, &["paired-token"]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        payload["error"],
        "Unauthorized — send a valid skey as Authorization: Bearer <skey>"
    );
}

#[tokio::test]
async fn handle_node_control_accepts_valid_pairing_bearer() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer paired-token"));

    let response = request(
        state_with(config_with_node_control(true, &[], None), true, &["paired-token"]),
        headers,
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["ok"], true);
}

#[tokio::test]
async fn handle_node_control_rejects_invalid_json_body() {
    let state = state_with(config_with_node_control(true, &[], None), false, &[]);
    let request = Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{"))
        .expect("request should be built");
    let body = axum::Json::<NodeControlRequest>::from_request(request, &state).await;
    let response = handle_node_control(State(state), HeaderMap::new(), body).await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"], "Invalid JSON body for node-control request");
}

#[tokio::test]
async fn handle_node_control_returns_not_found_when_disabled() {
    let response = request(
        state_with(config_with_node_control(false, &[], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(payload["error"], "Node-control API is disabled");
}

#[tokio::test]
async fn handle_node_control_rejects_invalid_node_token() {
    let response = request(
        state_with(config_with_node_control(true, &[], Some("shared-token")), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload["error"], "Invalid X-Node-Control-Token");
}

#[tokio::test]
async fn handle_node_control_accepts_trimmed_node_token() {
    let mut headers = HeaderMap::new();
    headers.insert("X-Node-Control-Token", HeaderValue::from_static(" shared-token "));

    let response = request(
        state_with(config_with_node_control(true, &[], Some(" shared-token ")), false, &[]),
        headers,
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["method"], "node.list");
}

#[tokio::test]
async fn handle_node_control_ignores_blank_node_token_config() {
    let response = request(
        state_with(config_with_node_control(true, &[], Some("   ")), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["method"], "node.list");
}

#[tokio::test]
async fn handle_node_control_rejects_non_utf8_node_token() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Node-Control-Token",
        HeaderValue::from_bytes(b"\xff").expect("opaque header value should be accepted"),
    );

    let response = request(
        state_with(config_with_node_control(true, &[], Some("shared-token")), false, &[]),
        headers,
        NodeControlRequest {
            method: "node.list".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload["error"], "Invalid X-Node-Control-Token");
}

#[tokio::test]
async fn handle_node_control_lists_configured_nodes() {
    let response = request(
        state_with(config_with_node_control(true, &["node-1", "node-2"], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: " node.list ".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["nodes"][0]["node_id"], "node-1");
    assert_eq!(payload["nodes"][1]["status"], "unpaired");
    assert_eq!(payload["nodes"][1]["capabilities"], serde_json::json!([]));
}

#[tokio::test]
async fn handle_node_control_describe_requires_node_id() {
    let response = request(
        state_with(config_with_node_control(true, &[], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.describe".to_string(),
            node_id: Some(" ".to_string()),
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"], "node_id is required for node.describe");
}

#[tokio::test]
async fn handle_node_control_describe_rejects_disallowed_node() {
    let response = request(
        state_with(config_with_node_control(true, &["node-1"], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.describe".to_string(),
            node_id: Some("node-2".to_string()),
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(payload["error"], "node_id is not allowed");
}

#[tokio::test]
async fn handle_node_control_describes_allowed_node() {
    let response = request(
        state_with(config_with_node_control(true, &["node-1"], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.describe".to_string(),
            node_id: Some(" node-1 ".to_string()),
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["node_id"], "node-1");
    assert_eq!(payload["description"]["status"], "stub");
    assert_eq!(
        payload["description"]["message"],
        "Node descriptor scaffold is enabled; runtime backend is not wired yet."
    );
}

#[tokio::test]
async fn handle_node_control_describes_node_when_allowlist_is_empty() {
    let response = request(
        state_with(config_with_node_control(true, &[], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.describe".to_string(),
            node_id: Some("node-without-entry".to_string()),
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["node_id"], "node-without-entry");
}

#[tokio::test]
async fn handle_node_control_invoke_requires_node_id() {
    let response = request(
        state_with(config_with_node_control(true, &[], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.invoke".to_string(),
            node_id: None,
            capability: Some("shell".to_string()),
            arguments: serde_json::json!({"command": "pwd"}),
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"], "node_id is required for node.invoke");
}

#[tokio::test]
async fn handle_node_control_invoke_rejects_disallowed_node() {
    let response = request(
        state_with(config_with_node_control(true, &["node-1"], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.invoke".to_string(),
            node_id: Some("node-2".to_string()),
            capability: Some("shell".to_string()),
            arguments: serde_json::json!({"command": "pwd"}),
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(payload["error"], "node_id is not allowed");
}

#[tokio::test]
async fn handle_node_control_invoke_returns_scaffold_error() {
    let arguments = serde_json::json!({"command": "pwd"});
    let response = request(
        state_with(config_with_node_control(true, &["*"], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.invoke".to_string(),
            node_id: Some(" node-2 ".to_string()),
            capability: Some("shell".to_string()),
            arguments: arguments.clone(),
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["node_id"], "node-2");
    assert_eq!(payload["capability"], "shell");
    assert_eq!(payload["arguments"], arguments);
    assert_eq!(payload["error"], "node.invoke backend is not implemented yet in this scaffold");
}

#[tokio::test]
async fn handle_node_control_invoke_preserves_missing_optional_fields() {
    let response = request(
        state_with(config_with_node_control(true, &["node-1"], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.invoke".to_string(),
            node_id: Some("node-1".to_string()),
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(payload["capability"], serde_json::Value::Null);
    assert_eq!(payload["arguments"], serde_json::Value::Null);
}

#[tokio::test]
async fn handle_node_control_rejects_unsupported_method() {
    let response = request(
        state_with(config_with_node_control(true, &[], None), false, &[]),
        HeaderMap::new(),
        NodeControlRequest {
            method: "node.remove".to_string(),
            node_id: None,
            capability: None,
            arguments: serde_json::Value::Null,
        },
    )
    .await;

    let (status, payload) = response_json(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"], "Unsupported method");
    assert_eq!(
        payload["supported_methods"],
        serde_json::json!(["node.list", "node.describe", "node.invoke"])
    );
}

async fn request(state: AppState, headers: HeaderMap, request: NodeControlRequest) -> Response {
    handle_node_control(State(state), headers, Ok(axum::Json(request))).await
}

async fn response_json(response: Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    let payload = serde_json::from_slice(&bytes).expect("response should be json");
    (status, payload)
}

fn state_with(config: Config, require_pairing: bool, paired_tokens: &[&str]) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    let paired_tokens = paired_tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>();

    AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, &paired_tokens)),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
    }
}

fn config_with_node_control(
    enabled: bool,
    allowed_node_ids: &[&str],
    auth_token: Option<&str>,
) -> Config {
    let mut config = Config::default();
    config.gateway.node_control.enabled = enabled;
    config.gateway.node_control.allowed_node_ids =
        allowed_node_ids.iter().map(|node_id| node_id.to_string()).collect();
    config.gateway.node_control.auth_token = auth_token.map(ToString::to_string);
    config
}
