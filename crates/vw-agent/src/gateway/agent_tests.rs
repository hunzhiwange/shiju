use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::{Memory, MemoryCategory, MemoryEntry, NoneMemory};
use crate::app::agent::observability::traits::{Observer, ObserverEvent, ObserverMetric};
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderValue, Method, Request, header};
use axum::routing::post;
use parking_lot::Mutex;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tower::util::ServiceExt;

struct StaticProvider;

struct FailingProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
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
        anyhow::bail!("provider failed with sk-test-secret")
    }
}

#[derive(Default)]
struct TrackingMemory {
    entries: Mutex<Vec<(String, String, MemoryCategory)>>,
    fail_store: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
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
        if self.fail_store {
            anyhow::bail!("intentional memory store failure");
        }
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

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<ObserverEvent>>,
}

impl RecordingObserver {
    fn events(&self) -> Vec<ObserverEvent> {
        self.events.lock().clone()
    }
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        self.events.lock().push(event.clone());
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "recording"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn peer() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3030)
}

fn state(require_pairing: bool, paired_tokens: &[String], agent_rate_limit: u32) -> AppState {
    let mut config = Config::default();
    config.memory.backend = "none".to_string();
    config.memory.hygiene_enabled = false;
    AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, paired_tokens)),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, agent_rate_limit, 100)),
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&body).expect("response should be json")
}

#[tokio::test]
async fn handle_agent_rejects_unauthorized_requests_before_body_validation() {
    let token = "paired-token".to_string();
    let response = handle_agent(
        State(state(true, std::slice::from_ref(&token), 100)),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "hello".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Unauthorized"));
}

#[tokio::test]
async fn handle_agent_accepts_valid_bearer_before_body_validation() {
    let token = "paired-token".to_string();
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer paired-token"));

    let response = handle_agent(
        State(state(true, &[token], 100)),
        ConnectInfo(peer()),
        headers,
        Ok(Json(AgentBody { message: " \n\t".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_json(response).await["error"], "message must not be empty");
}

#[tokio::test]
async fn handle_agent_rejects_invalid_json_body() {
    let app = Router::new().route("/agent", post(handle_agent)).with_state(state(false, &[], 100));
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/agent")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{ invalid json"))
        .expect("request should build");
    request.extensions_mut().insert(ConnectInfo(peer()));

    let response = app.oneshot(request).await.expect("router should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(response.into_response()).await["error"]
            .as_str()
            .unwrap()
            .contains("Invalid JSON")
    );
}

#[tokio::test]
async fn handle_agent_rejects_empty_message() {
    let response = handle_agent(
        State(state(false, &[], 100)),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "   \n\t".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_json(response).await["error"], "message must not be empty");
}

#[tokio::test]
async fn handle_agent_enforces_agent_rate_limit_before_auth() {
    let state = state(false, &[], 1);
    let body = || Ok(Json(AgentBody { message: String::new() }));

    let first =
        handle_agent(State(state.clone()), ConnectInfo(peer()), HeaderMap::new(), body()).await;
    let second = handle_agent(State(state), ConnectInfo(peer()), HeaderMap::new(), body()).await;

    assert_eq!(first.status(), StatusCode::BAD_REQUEST);
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response_json(second).await["retry_after"], RATE_LIMIT_WINDOW_SECS);
}

#[tokio::test]
async fn handle_agent_uses_trusted_forwarded_client_for_rate_limit() {
    let mut state = state(false, &[], 1);
    state.trust_forwarded_headers = true;
    let mut first_headers = HeaderMap::new();
    first_headers.insert("x-forwarded-for", HeaderValue::from_static("198.51.100.10"));
    let mut second_headers = HeaderMap::new();
    second_headers.insert("x-forwarded-for", HeaderValue::from_static("198.51.100.11"));

    let first = handle_agent(
        State(state.clone()),
        ConnectInfo(peer()),
        first_headers,
        Ok(Json(AgentBody { message: String::new() })),
    )
    .await;
    let second = handle_agent(
        State(state),
        ConnectInfo(peer()),
        second_headers,
        Ok(Json(AgentBody { message: String::new() })),
    )
    .await;

    assert_eq!(first.status(), StatusCode::BAD_REQUEST);
    assert_eq!(second.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn handle_agent_returns_bad_gateway_when_agent_engine_fails() {
    let mut state = state(false, &[], 100);
    state.provider = Arc::new(FailingProvider);
    state.config.lock().default_provider = Some("unsupported-provider".to_string());

    let response = handle_agent(
        State(state),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "hello".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Provider error"));
}

#[tokio::test]
async fn handle_agent_auto_saves_trimmed_message_and_ignores_store_failure() {
    let memory = Arc::new(TrackingMemory { fail_store: true, ..TrackingMemory::default() });
    let mut state = state(false, &[], 100);
    state.provider = Arc::new(FailingProvider);
    state.mem = memory.clone();
    state.auto_save = true;
    state.config.lock().default_provider = Some("unsupported-provider".to_string());

    let response = handle_agent(
        State(state),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "  remember this  \n".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let entries = memory.entries.lock();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].0.starts_with("webhook_msg_"));
    assert_eq!(entries[0].1, "remember this");
    assert_eq!(entries[0].2, MemoryCategory::Conversation);
}

#[tokio::test]
async fn handle_agent_records_failure_events_with_default_labels() {
    let observer = Arc::new(RecordingObserver::default());
    let mut state = state(false, &[], 100);
    state.observer = observer.clone();
    state.model = "endpoint-model".to_string();
    state.config.lock().default_provider = None;
    state.config.lock().memory.backend = "mariadb".to_string();

    let response = handle_agent(
        State(state),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "hello".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let events = observer.events();
    assert!(
        matches!(
            events.as_slice(),
            [
                ObserverEvent::AgentStart { provider, model },
                ObserverEvent::LlmRequest {
                    provider: request_provider,
                    model: request_model,
                    messages_count: 1,
                },
                ObserverEvent::LlmResponse {
                    provider: response_provider,
                    model: response_model,
                    success: false,
                    error_message: Some(_),
                    ..
                },
            ] if provider == "unknown"
                && model == "endpoint-model"
                && request_provider == provider
                && request_model == model
                && response_provider == provider
                && response_model == model
        ),
        "unexpected observer events: {events:?}"
    );
}

#[tokio::test]
async fn handle_agent_sanitizes_secrets_in_provider_errors() {
    let mut state = state(false, &[], 100);
    state.provider = Arc::new(FailingProvider);
    state.config.lock().default_provider = Some("sk-test-secret".to_string());

    let response = handle_agent(
        State(state),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "hello".to_string() })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response_json(response).await;
    let error = body["error"].as_str().expect("error should be a string");
    assert!(error.contains("[REDACTED]"));
    assert!(!error.contains("sk-test-secret"));
}

#[tokio::test]
async fn handle_agent_returns_response_and_records_complete_success_lifecycle() {
    let observer = Arc::new(RecordingObserver::default());
    let mut state = state(false, &[], 100);
    state.observer = observer.clone();
    state.config.lock().default_provider = Some("static".to_string());

    let response = handle_agent(
        State(state),
        ConnectInfo(peer()),
        HeaderMap::new(),
        Ok(Json(AgentBody { message: "hello".to_string() })),
    )
    .await;

    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "unexpected response: {body}");
    assert_eq!(body["response"], "ok");
    let events = observer.events();
    assert!(matches!(
        events.as_slice(),
        [
            ObserverEvent::AgentStart { provider, model },
            ObserverEvent::LlmRequest {
                provider: request_provider,
                model: request_model,
                messages_count: 1,
            },
            ObserverEvent::LlmResponse {
                provider: response_provider,
                model: response_model,
                success: true,
                error_message: None,
                ..
            },
            ObserverEvent::TurnComplete,
            ObserverEvent::AgentEnd {
                provider: end_provider,
                model: end_model,
                ..
            },
        ] if provider == "static"
            && model == "test-model"
            && request_provider == provider
            && request_model == model
            && response_provider == provider
            && response_model == model
            && end_provider == provider
            && end_model == model
    ));
}
