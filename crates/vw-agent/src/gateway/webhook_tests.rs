// 验证 Webhook 请求的认证、去重、渠道分派与响应契约。
use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore, hash_webhook_secret};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::body::to_bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, header};
use axum::routing::post;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;
use tower::ServiceExt;

#[test]
fn webhook_handler_is_available() {
    let _ = handle_webhook;
}

struct CountingProvider {
    calls: Arc<AtomicUsize>,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for CountingProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(format!("handled: {message}"))
    }
}

struct FailingProvider {
    calls: Arc<AtomicUsize>,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for FailingProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("provider failed with test secret token");
    }
}

fn test_state_with_provider(
    provider: Arc<dyn Provider>,
    webhook_secret_hash: Option<Arc<str>>,
) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider,
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash,
        pairing: Arc::new(PairingGuard::new(false, &[])),
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
        max_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn test_state(calls: Arc<AtomicUsize>, webhook_secret_hash: Option<Arc<str>>) -> AppState {
    test_state_with_provider(Arc::new(CountingProvider { calls }), webhook_secret_hash)
}

fn peer(ip: IpAddr) -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::new(ip, 8080))
}

#[tokio::test]
async fn handle_webhook_accepts_loopback_request() {
    let calls = Arc::new(AtomicUsize::new(0));
    let response = handle_webhook(
        State(test_state(Arc::clone(&calls), None)),
        peer(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        HeaderMap::new(),
        Ok(Json(WebhookBody { message: "ping".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["response"], "handled: ping");
}

#[tokio::test]
async fn handle_webhook_rejects_public_request_without_auth() {
    let calls = Arc::new(AtomicUsize::new(0));
    let response = handle_webhook(
        State(test_state(Arc::clone(&calls), None)),
        peer(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))),
        HeaderMap::new(),
        Ok(Json(WebhookBody { message: "ping".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn handle_webhook_accepts_valid_secret_header() {
    let secret = "test-webhook-secret";
    let calls = Arc::new(AtomicUsize::new(0));
    let mut headers = HeaderMap::new();
    headers.insert("X-Webhook-Secret", HeaderValue::from_static(secret));

    let response = handle_webhook(
        State(test_state(Arc::clone(&calls), Some(Arc::from(hash_webhook_secret(secret))))),
        peer(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))),
        headers,
        Ok(Json(WebhookBody { message: "secret ping".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handle_webhook_returns_bad_request_for_json_rejection() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(test_state(Arc::clone(&calls), None));
    let mut request = Request::post("/webhook")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"message\":"))
        .unwrap();
    request.extensions_mut().insert(peer(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "Invalid JSON body. Expected: {\"message\": \"...\"}");
}

#[tokio::test]
async fn handle_webhook_maps_provider_errors_to_internal_server_error() {
    let calls = Arc::new(AtomicUsize::new(0));
    let response = handle_webhook(
        State(test_state_with_provider(
            Arc::new(FailingProvider { calls: Arc::clone(&calls) }),
            None,
        )),
        peer(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        HeaderMap::new(),
        Ok(Json(WebhookBody { message: "ping".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "LLM request failed");
}
