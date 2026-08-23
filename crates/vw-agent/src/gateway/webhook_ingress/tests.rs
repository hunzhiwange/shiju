// 覆盖 Webhook 入口的鉴权、限流、去重、载荷解析与错误脱敏。

// 认证、限流和幂等检查必须先于 provider 调用，失败时也不得泄漏内部详情。
use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry, NoneMemory};
use crate::app::agent::observability::traits::{Observer, ObserverEvent, ObserverMetric};
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::FromRequest,
    http::{HeaderValue, Request, header},
};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Default)]
// provider 桩记录调用次数，用于证明认证或限流失败确实在模型调用前短路。
struct StaticProvider {
    calls: AtomicUsize,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(format!("{model}:{message}"))
    }
}

struct FailingProvider;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for FailingProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("provider token sk-test-secret failed")
    }
}

#[derive(Default)]
struct FailingStoreMemory {
    store_calls: AtomicUsize,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for FailingStoreMemory {
    fn name(&self) -> &str {
        "failing-store"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.store_calls.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("memory store unavailable")
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    async fn health_check(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct TrackingMemory {
    entries: Mutex<Vec<(String, String, MemoryCategory)>>,
}

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<ObserverEvent>>,
    metrics: Mutex<Vec<ObserverMetric>>,
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().push(event.clone());
    }

    fn record_metric(&self, metric: &ObserverMetric) {
        self.metrics.lock().push(metric.clone());
    }

    fn name(&self) -> &str {
        "recording"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for TrackingMemory {
    fn name(&self) -> &str {
        "tracking"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.entries.lock().push((key.to_string(), content.to_string(), category));
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.entries.lock().len())
    }

    async fn health_check(&self) -> bool {
        true
    }
}

// 客户端地址显式构造，便于验证限流键和回环授权不会混用来源。
fn peer(ip: &str) -> SocketAddr {
    format!("{ip}:30300").parse().expect("test socket address should parse")
}

fn state_with(
    provider: Arc<dyn Provider>,
    mem: Arc<dyn Memory>,
    pairing: PairingGuard,
    webhook_secret_hash: Option<Arc<str>>,
    webhook_rate_limit: u32,
    auto_save: bool,
) -> AppState {
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider,
        model: "test-model".to_string(),
        temperature: 0.0,
        mem,
        auto_save,
        webhook_secret_hash,
        pairing: Arc::new(pairing),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, webhook_rate_limit, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 100)),
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn basic_state() -> AppState {
    state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    )
}

fn payload(response: (StatusCode, Json<Value>)) -> (StatusCode, Value) {
    (response.0, response.1.0)
}

#[test]
fn extract_idempotency_key_handles_missing_blank_trimmed_and_invalid_values() {
    let mut headers = HeaderMap::new();
    assert_eq!(extract_idempotency_key(&headers), None);

    headers.insert("X-Idempotency-Key", HeaderValue::from_static(""));
    assert_eq!(extract_idempotency_key(&headers), None);

    headers.insert("X-Idempotency-Key", HeaderValue::from_static("  request-123  "));
    assert_eq!(extract_idempotency_key(&headers), Some("request-123"));

    headers.insert(
        "X-Idempotency-Key",
        HeaderValue::from_bytes(b"\xff").expect("invalid utf-8 header value should build"),
    );
    assert_eq!(extract_idempotency_key(&headers), None);
}

#[test]
fn extract_webhook_secret_header_hash_trims_and_hashes_nonempty_secret() {
    let mut headers = HeaderMap::new();
    assert_eq!(extract_webhook_secret_header_hash(&headers), None);

    headers.insert("X-Webhook-Secret", HeaderValue::from_static("   "));
    assert_eq!(extract_webhook_secret_header_hash(&headers), None);

    headers.insert("X-Webhook-Secret", HeaderValue::from_static("  shared-secret  "));
    assert_eq!(
        extract_webhook_secret_header_hash(&headers),
        Some(hash_webhook_secret("shared-secret"))
    );

    headers.insert(
        "X-Webhook-Secret",
        HeaderValue::from_bytes(b"\xff").expect("invalid utf-8 header value should build"),
    );
    assert_eq!(extract_webhook_secret_header_hash(&headers), None);
}

#[test]
fn parse_webhook_body_accepts_valid_json_body() {
    let parsed = parse_webhook_body(Ok(Json(WebhookBody { message: "hello".to_string() })))
        .expect("valid body should parse");

    assert_eq!(parsed.message, "hello");
}

#[tokio::test]
async fn parse_webhook_body_rejects_invalid_json_body() {
    let request = Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{"))
        .expect("request should build");
    let rejected = Json::<WebhookBody>::from_request(request, &()).await;

    let response = match parse_webhook_body(rejected) {
        Ok(_) => panic!("body should reject"),
        Err(response) => response,
    };
    let (status, body) = payload(response);
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON body. Expected: {\"message\": \"...\"}");
}

#[test]
fn enforce_idempotency_allows_missing_and_first_key_then_rejects_duplicate() {
    let state = basic_state();
    let mut headers = HeaderMap::new();

    assert!(enforce_idempotency(&state, &headers).is_none());

    headers.insert("X-Idempotency-Key", HeaderValue::from_static("same-request"));
    assert!(enforce_idempotency(&state, &headers).is_none());

    let (status, body) = payload(
        enforce_idempotency(&state, &headers).expect("duplicate key should return response"),
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "duplicate");
    assert_eq!(body["idempotent"], true);
}

#[test]
fn enforce_idempotency_treats_trimmed_equivalent_keys_as_duplicates() {
    let state = basic_state();
    let mut headers = HeaderMap::new();
    headers.insert("X-Idempotency-Key", HeaderValue::from_static("  normalized-key  "));

    assert!(enforce_idempotency(&state, &headers).is_none());

    headers.insert("X-Idempotency-Key", HeaderValue::from_static("normalized-key"));
    let (status, body) = payload(
        enforce_idempotency(&state, &headers).expect("trim-equivalent key should be duplicate"),
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "duplicate");
    assert_eq!(body["message"], "Request already processed for this idempotency key");
}

#[test]
fn enforce_rate_limit_returns_retry_after_after_limit_is_exceeded() {
    let state = state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        1,
        false,
    );
    let headers = HeaderMap::new();

    assert!(enforce_rate_limit(&state, peer("127.0.0.1"), &headers).is_none());
    let (status, body) = payload(
        enforce_rate_limit(&state, peer("127.0.0.1"), &headers)
            .expect("second request should be rate limited"),
    );

    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
}

#[test]
fn enforce_rate_limit_scopes_limits_to_each_peer_address() {
    let state = state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        1,
        false,
    );
    let headers = HeaderMap::new();
    let first_peer = peer("192.0.2.10");
    let second_peer = peer("192.0.2.11");

    assert!(enforce_rate_limit(&state, first_peer, &headers).is_none());
    assert!(enforce_rate_limit(&state, second_peer, &headers).is_none());
    assert!(enforce_rate_limit(&state, first_peer, &headers).is_some());
    assert!(enforce_rate_limit(&state, second_peer, &headers).is_some());
}

#[test]
fn authorize_webhook_request_allows_loopback_without_auth_and_rejects_remote() {
    let state = basic_state();

    assert!(authorize_webhook_request(&state, peer("127.0.0.1"), &HeaderMap::new()).is_none());

    let (status, body) = payload(
        authorize_webhook_request(&state, peer("203.0.113.10"), &HeaderMap::new())
            .expect("remote request without auth should reject"),
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body["error"].as_str().expect("error should be string").contains("configure gateway auth")
    );
}

#[test]
fn authorize_webhook_request_requires_valid_bearer_when_pairing_is_enabled() {
    let paired_token = "paired-token".to_string();
    let state = state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(true, std::slice::from_ref(&paired_token)),
        None,
        100,
        false,
    );
    let mut headers = HeaderMap::new();

    let (status, _) = payload(
        authorize_webhook_request(&state, peer("127.0.0.1"), &headers)
            .expect("missing bearer should reject"),
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer paired-token"));
    assert!(authorize_webhook_request(&state, peer("203.0.113.10"), &headers).is_none());
}

#[test]
fn authorize_webhook_request_requires_matching_webhook_secret_when_configured() {
    let state = state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        Some(Arc::from(hash_webhook_secret("shared-secret"))),
        100,
        false,
    );
    let mut headers = HeaderMap::new();

    let (status, _) = payload(
        authorize_webhook_request(&state, peer("127.0.0.1"), &headers)
            .expect("missing secret should reject"),
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    headers.insert("X-Webhook-Secret", HeaderValue::from_static("shared-secret"));
    assert!(authorize_webhook_request(&state, peer("203.0.113.10"), &headers).is_none());
}

#[test]
fn authorize_webhook_request_requires_both_credentials_when_both_are_configured() {
    let paired_token = "paired-token".to_string();
    let state = state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(true, std::slice::from_ref(&paired_token)),
        Some(Arc::from(hash_webhook_secret("shared-secret"))),
        100,
        false,
    );
    let mut headers = HeaderMap::new();
    headers.insert("X-Webhook-Secret", HeaderValue::from_static("shared-secret"));

    let (status, body) = payload(
        authorize_webhook_request(&state, peer("203.0.113.10"), &headers)
            .expect("webhook secret alone should not bypass gateway auth"),
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().expect("error should be string").contains("valid skey"));

    headers.remove("X-Webhook-Secret");
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer paired-token"));
    let (status, body) = payload(
        authorize_webhook_request(&state, peer("203.0.113.10"), &headers)
            .expect("gateway auth alone should not bypass webhook secret"),
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().expect("error should be string").contains("X-Webhook-Secret"));

    headers.insert("X-Webhook-Secret", HeaderValue::from_static("shared-secret"));
    assert!(authorize_webhook_request(&state, peer("203.0.113.10"), &headers).is_none());
}

#[tokio::test]
async fn maybe_persist_inbound_message_skips_when_auto_save_is_disabled() {
    let tracking = Arc::new(TrackingMemory::default());
    let state = state_with(
        Arc::new(StaticProvider::default()),
        tracking.clone(),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );

    maybe_persist_inbound_message(&state, "hello").await;

    assert!(tracking.entries.lock().is_empty());
}

#[tokio::test]
async fn maybe_persist_inbound_message_stores_conversation_when_auto_save_is_enabled() {
    let tracking = Arc::new(TrackingMemory::default());
    let state = state_with(
        Arc::new(StaticProvider::default()),
        tracking.clone(),
        PairingGuard::new(false, &[]),
        None,
        100,
        true,
    );

    maybe_persist_inbound_message(&state, "hello").await;

    let entries = tracking.entries.lock();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].0.starts_with("webhook_msg_"));
    assert_eq!(entries[0].1, "hello");
    assert_eq!(entries[0].2, MemoryCategory::Conversation);
}

#[tokio::test]
async fn handle_webhook_inner_continues_when_auto_save_store_fails() {
    let provider = Arc::new(StaticProvider::default());
    let memory = Arc::new(FailingStoreMemory::default());
    let state = state_with(
        provider.clone(),
        memory.clone(),
        PairingGuard::new(false, &[]),
        None,
        100,
        true,
    );

    let (status, body) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            HeaderMap::new(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["response"], "test-model:hello");
    assert_eq!(memory.store_calls.load(Ordering::SeqCst), 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handle_webhook_inner_returns_success_payload_for_authorized_loopback_request() {
    let provider = Arc::new(StaticProvider::default());
    let state = state_with(
        provider.clone(),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );

    let (status, body) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            HeaderMap::new(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["response"], "test-model:hello");
    assert_eq!(body["model"], "test-model");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handle_webhook_inner_returns_rate_limit_before_provider_call() {
    let provider = Arc::new(StaticProvider::default());
    let state = state_with(
        provider.clone(),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        1,
        false,
    );
    let headers = HeaderMap::new();

    let (first_status, _) = payload(
        handle_webhook_inner(
            state.clone(),
            peer("127.0.0.1"),
            headers.clone(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );
    let (second_status, body) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            headers,
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handle_webhook_inner_returns_auth_error_before_provider_call() {
    let provider = Arc::new(StaticProvider::default());
    let state = state_with(
        provider.clone(),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );

    let (status, body) = payload(
        handle_webhook_inner(
            state,
            peer("203.0.113.10"),
            HeaderMap::new(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        body["error"].as_str().expect("error should be string").contains("configure gateway auth")
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn handle_webhook_inner_returns_bad_request_before_provider_call() {
    let provider = Arc::new(StaticProvider::default());
    let state = state_with(
        provider.clone(),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );
    let request = Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{"))
        .expect("request should build");
    let rejected = Json::<WebhookBody>::from_request(request, &()).await;

    let (status, body) =
        payload(handle_webhook_inner(state, peer("127.0.0.1"), HeaderMap::new(), rejected).await);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON body. Expected: {\"message\": \"...\"}");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn handle_webhook_inner_returns_duplicate_without_second_provider_call() {
    let provider = Arc::new(StaticProvider::default());
    let state = state_with(
        provider.clone(),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );
    let mut headers = HeaderMap::new();
    headers.insert("X-Idempotency-Key", HeaderValue::from_static("same-request"));

    let (first_status, _) = payload(
        handle_webhook_inner(
            state.clone(),
            peer("127.0.0.1"),
            headers.clone(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );
    let (second_status, body) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            headers,
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(body["status"], "duplicate");
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handle_webhook_inner_returns_provider_error_without_leaking_details() {
    let state = state_with(
        Arc::new(FailingProvider),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );

    let (status, body) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            HeaderMap::new(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["error"], "LLM request failed");
}

#[tokio::test]
async fn handle_webhook_inner_records_success_telemetry() {
    let observer = Arc::new(RecordingObserver::default());
    let mut state = state_with(
        Arc::new(StaticProvider::default()),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );
    state.config.lock().default_provider = Some("configured-provider".to_string());
    state.observer = observer.clone();

    let (status, _) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            HeaderMap::new(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(status, StatusCode::OK);
    let events = observer.events.lock();
    assert!(matches!(
        &events[0],
        ObserverEvent::AgentStart { provider, model }
            if provider == "configured-provider" && model == "test-model"
    ));
    assert!(matches!(
        &events[1],
        ObserverEvent::LlmRequest { provider, model, messages_count }
            if provider == "configured-provider" && model == "test-model" && *messages_count == 1
    ));
    assert!(matches!(
        &events[2],
        ObserverEvent::LlmResponse { provider, model, success, error_message, .. }
            if provider == "configured-provider"
                && model == "test-model"
                && *success
                && error_message.is_none()
    ));
    assert!(matches!(
        &events[3],
        ObserverEvent::AgentEnd { provider, model, tokens_used, cost_usd, .. }
            if provider == "configured-provider"
                && model == "test-model"
                && tokens_used.is_none()
                && cost_usd.is_none()
    ));
    assert!(matches!(observer.metrics.lock().as_slice(), [ObserverMetric::RequestLatency(_)]));
}

#[tokio::test]
async fn handle_webhook_inner_records_error_telemetry_with_unknown_provider() {
    let observer = Arc::new(RecordingObserver::default());
    let mut state = state_with(
        Arc::new(FailingProvider),
        Arc::new(NoneMemory::new()),
        PairingGuard::new(false, &[]),
        None,
        100,
        false,
    );
    state.config.lock().default_provider = None;
    state.observer = observer.clone();

    let (status, _) = payload(
        handle_webhook_inner(
            state,
            peer("127.0.0.1"),
            HeaderMap::new(),
            Ok(Json(WebhookBody { message: "hello".to_string() })),
        )
        .await,
    );

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let events = observer.events.lock();
    assert!(matches!(
        &events[0],
        ObserverEvent::AgentStart { provider, model }
            if provider == "unknown" && model == "test-model"
    ));
    assert!(matches!(
        &events[2],
        ObserverEvent::LlmResponse { provider, model, success, error_message, .. }
            if provider == "unknown"
                && model == "test-model"
                && !*success
                && error_message.as_deref() == Some("provider token [REDACTED] failed")
    ));
    assert!(matches!(
        &events[3],
        ObserverEvent::Error { component, message }
            if component == "gateway" && message == "provider token [REDACTED] failed"
    ));
    assert!(matches!(
        &events[4],
        ObserverEvent::AgentEnd { provider, model, tokens_used, cost_usd, .. }
            if provider == "unknown"
                && model == "test-model"
                && tokens_used.is_none()
                && cost_usd.is_none()
    ));
    assert!(matches!(observer.metrics.lock().as_slice(), [ObserverMetric::RequestLatency(_)]));
}
