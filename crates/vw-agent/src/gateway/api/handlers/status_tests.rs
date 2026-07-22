use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::config::schema::WebhookConfig;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
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
    let mut config = Config::default();
    config.gateway.port = 49123;
    config.default_provider = Some("test-provider".to_string());
    let paired_tokens = paired_tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>();

    AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.25,
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

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("body should be json")
}

#[test]
fn status_handlers_are_available() {
    let _ = handle_api_status;
    let _ = handle_api_health;
}

#[tokio::test]
async fn handle_api_status_returns_runtime_status_when_authorized() {
    let response =
        handle_api_status(State(state(false, &[])), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["provider"], "test-provider");
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["temperature"], 0.25);
    assert_eq!(body["gateway_port"], 49123);
    assert_eq!(body["locale"], "en");
    assert_eq!(body["memory_backend"], "none");
    assert_eq!(body["auth_enabled"], false);
    assert_eq!(body["active_skeys"], 0);
    assert!(body["channels"].is_object());
    assert!(body["health"].is_object());
    assert!(body["uptime_seconds"].as_u64().is_some());
    assert_eq!(body["uptime_seconds"], body["health"]["uptime_seconds"]);
}

#[tokio::test]
async fn handle_api_status_reports_channel_presence_and_absent_provider() {
    let state = state(false, &[]);
    {
        let mut config = state.config.lock();
        config.default_provider = None;
        config.channels_config.webhook = Some(WebhookConfig { port: 8787, secret: None });
    }

    let response = handle_api_status(State(state), HeaderMap::new()).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert!(body["provider"].is_null());
    assert_eq!(body["channels"]["Webhook"], true);
    assert_eq!(body["channels"]["Telegram"], false);
}

#[tokio::test]
async fn handle_api_health_returns_health_snapshot_when_authorized() {
    let response =
        handle_api_health(State(state(false, &[])), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert!(body["health"].is_object());
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert!(body["health"]["pid"].as_u64().is_some());
    assert!(body["health"]["updated_at"].as_str().is_some());
    assert!(body["health"]["uptime_seconds"].as_u64().is_some());
    assert!(body["health"]["components"].is_object());
}

#[tokio::test]
async fn status_and_health_reject_missing_bearer_when_skey_auth_required() {
    let status = handle_api_status(State(state(true, &["paired-token"])), HeaderMap::new())
        .await
        .into_response();
    assert_eq!(status.status(), StatusCode::UNAUTHORIZED);
    assert!(response_json(status).await["error"].as_str().unwrap().contains("Unauthorized"));

    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer wrong-token".parse().unwrap());
    let health =
        handle_api_health(State(state(true, &["paired-token"])), headers).await.into_response();
    assert_eq!(health.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn status_accepts_valid_bearer_skey() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer paired-token".parse().unwrap());

    let response =
        handle_api_status(State(state(true, &["paired-token"])), headers).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["auth_enabled"], true);
    assert_eq!(body["active_skeys"], 1);
}

#[tokio::test]
async fn health_accepts_valid_bearer_skey() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer paired-token".parse().unwrap());

    let response =
        handle_api_health(State(state(true, &["paired-token"])), headers).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_json(response).await["health"].is_object());
}
