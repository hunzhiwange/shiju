use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::ToolSpec;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
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

fn state(require_pairing: bool, paired_tokens: &[&str]) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    let paired_tokens = paired_tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>();

    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
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
        tools_registry: Arc::new(vec![
            ToolSpec::new("unit_tool", "A unit test tool", serde_json::json!({"type": "object"}))
                .with_display_name("Unit Tool")
                .with_aliases(["unit"])
                .with_read_only(true),
        ]),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
    }
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("body should be json")
}

async fn assert_unauthorized(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response_json(response).await,
        serde_json::json!({
            "error": "Unauthorized — send a valid skey as Authorization: Bearer <skey>"
        })
    );
}

#[test]
fn tool_handlers_are_available() {
    let _ = handle_api_tools;
    let _ = handle_api_cli_tools;
    let _ = handle_api_doctor;
}

#[tokio::test]
async fn handle_api_tools_returns_registered_specs_when_authorized() {
    let response =
        handle_api_tools(State(state(false, &[])), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["items"][0]["id"], "unit_tool");
    assert_eq!(body["items"][0]["display_name"], "Unit Tool");
    assert_eq!(body["items"][0]["aliases"], serde_json::json!(["unit"]));
    assert_eq!(body["items"][0]["read_only"], true);
}

#[tokio::test]
async fn handle_api_tools_returns_empty_items_for_empty_registry() {
    let mut app_state = state(false, &[]);
    app_state.tools_registry = Arc::new(Vec::new());

    let response = handle_api_tools(State(app_state), HeaderMap::new()).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!({ "items": [] }));
}

#[tokio::test]
async fn handle_api_tools_preserves_registry_order_and_capabilities() {
    let mut app_state = state(false, &[]);
    app_state.tools_registry = Arc::new(vec![
        ToolSpec::new("reader", "Reads data", serde_json::json!({ "type": "object" }))
            .with_read_only(true)
            .with_concurrency_safe(true),
        ToolSpec::new("writer", "Writes data", serde_json::json!({ "type": "object" }))
            .with_destructive(true)
            .with_requires_user_interaction(true)
            .with_strict(false),
    ]);

    let response = handle_api_tools(State(app_state), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    let items = body["items"].as_array().expect("items should be an array");

    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "reader");
    assert_eq!(items[0]["read_only"], true);
    assert_eq!(items[0]["concurrency_safe"], true);
    assert_eq!(items[1]["id"], "writer");
    assert_eq!(items[1]["destructive"], true);
    assert_eq!(items[1]["requires_user_interaction"], true);
    assert_eq!(items[1]["strict"], false);
}

#[tokio::test]
async fn handle_api_cli_tools_returns_discovery_array_when_authorized() {
    let response =
        handle_api_cli_tools(State(state(false, &[])), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert!(body["cli_tools"].is_array());
    assert_eq!(body.as_object().expect("body should be an object").len(), 1);
}

#[tokio::test]
async fn handle_api_doctor_returns_results_and_summary_when_authorized() {
    let response =
        handle_api_doctor(State(state(false, &[])), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    let results = body["results"].as_array().expect("results should be an array");
    let summary = &body["summary"];
    let ok_count = results.iter().filter(|result| result["severity"] == "ok").count() as u64;
    let warning_count = results.iter().filter(|result| result["severity"] == "warn").count() as u64;
    let error_count = results.iter().filter(|result| result["severity"] == "error").count() as u64;

    assert_eq!(summary["ok"].as_u64(), Some(ok_count));
    assert_eq!(summary["warnings"].as_u64(), Some(warning_count));
    assert_eq!(summary["errors"].as_u64(), Some(error_count));
}

#[tokio::test]
async fn tool_handlers_reject_missing_or_wrong_bearer_when_pairing_required() {
    let tools = handle_api_tools(State(state(true, &["paired-token"])), HeaderMap::new())
        .await
        .into_response();
    assert_unauthorized(tools).await;

    let cli_tools = handle_api_cli_tools(State(state(true, &["paired-token"])), HeaderMap::new())
        .await
        .into_response();
    assert_unauthorized(cli_tools).await;

    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer wrong-token".parse().unwrap());
    let doctor =
        handle_api_doctor(State(state(true, &["paired-token"])), headers).await.into_response();
    assert_unauthorized(doctor).await;
}

#[tokio::test]
async fn tool_handlers_accept_valid_pairing_bearer() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer paired-token".parse().unwrap());

    let response =
        handle_api_tools(State(state(true, &["paired-token"])), headers).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
}
