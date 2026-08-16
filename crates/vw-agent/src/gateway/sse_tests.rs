use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::{Observer, ObserverEvent};
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::observability::traits::ObserverMetric;
use axum::body::to_bytes;
use axum::http::header;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Clone, Default)]
struct TestObserver {
    state: Arc<parking_lot::Mutex<TestObserverState>>,
}

#[derive(Default)]
struct TestObserverState {
    events: usize,
    metrics: usize,
    flushes: usize,
}

impl Observer for TestObserver {
    fn record_event(&self, _event: &ObserverEvent) {
        self.state.lock().events += 1;
    }

    fn record_metric(&self, _metric: &ObserverMetric) {
        self.state.lock().metrics += 1;
    }

    fn flush(&self) {
        self.state.lock().flushes += 1;
    }

    fn name(&self) -> &str {
        "test"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct StaticProvider;

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

fn state_with_pairing(
    event_tx: broadcast::Sender<serde_json::Value>,
    require_pairing: bool,
    paired_tokens: &[String],
) -> AppState {
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, paired_tokens)),
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
        hook_runtime: None,
    }
}

#[test]
fn broadcast_observer_has_stable_name() {
    let (sender, _receiver) = tokio::sync::broadcast::channel(4);
    let observer = BroadcastObserver::new(Box::new(TestObserver::default()), sender);

    assert_eq!(observer.name(), "gateway_broadcast");
    assert!(observer.as_any().is::<BroadcastObserver>());
}

#[tokio::test]
async fn handle_sse_events_rejects_missing_bearer_when_skey_auth_is_required() {
    let (sender, _receiver) = broadcast::channel(4);
    let state = state_with_pairing(sender, true, &[String::from("paired-token")]);

    let response = handle_sse_events(State(state), HeaderMap::new()).await.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.as_ref(), b"Unauthorized \xE2\x80\x94 provide Authorization: Bearer <skey>");
}

#[tokio::test]
async fn handle_sse_events_rejects_invalid_bearer_when_skey_auth_is_required() {
    let (sender, _receiver) = broadcast::channel(4);
    let state = state_with_pairing(sender, true, &[String::from("paired-token")]);
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer wrong-token".parse().unwrap());

    let response = handle_sse_events(State(state), headers).await.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.as_ref(), b"Unauthorized \xE2\x80\x94 provide Authorization: Bearer <skey>");
}

#[tokio::test]
async fn handle_sse_events_streams_json_data_without_pairing() {
    let (sender, _receiver) = broadcast::channel(4);
    let state = state_with_pairing(sender.clone(), false, &[]);

    let response = handle_sse_events(State(state), HeaderMap::new()).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    sender.send(serde_json::json!({"type": "agent_start"})).expect("event should send");
    drop(sender);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");

    assert_eq!(body.as_ref(), b"data: {\"type\":\"agent_start\"}\n\n");
}

#[tokio::test]
async fn handle_sse_events_ignores_invalid_bearer_when_skey_auth_is_disabled() {
    let (sender, _receiver) = broadcast::channel(4);
    let state = state_with_pairing(sender.clone(), false, &[]);
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer unpaired-token".parse().unwrap());

    let response = handle_sse_events(State(state), headers).await.into_response();
    let status = response.status();
    drop(sender);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
}

#[tokio::test]
async fn handle_sse_events_accepts_valid_bearer_and_skips_lagged_messages() {
    let (sender, _receiver) = broadcast::channel(1);
    let token = String::from("paired-token");
    let state = state_with_pairing(sender.clone(), true, std::slice::from_ref(&token));
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer paired-token".parse().unwrap());

    let response = handle_sse_events(State(state), headers).await.into_response();
    sender.send(serde_json::json!({"sequence": 1})).expect("first event should send");
    sender.send(serde_json::json!({"sequence": 2})).expect("second event should send");
    drop(sender);
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_ref(), b"data: {\"sequence\":2}\n\n");
}

#[test]
fn broadcast_observer_records_inner_observer_and_broadcasts_supported_events() {
    let (sender, mut receiver) = broadcast::channel(8);
    let inner = TestObserver::default();
    let inner_state = inner.state.clone();
    let observer = BroadcastObserver::new(Box::new(inner), sender);

    let cases = [
        (
            ObserverEvent::LlmRequest {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                messages_count: 3,
            },
            serde_json::json!({"type": "llm_request", "provider": "openai", "model": "gpt-4"}),
        ),
        (
            ObserverEvent::ToolCall {
                tool: "shell".to_string(),
                duration: Duration::from_millis(42),
                success: true,
            },
            serde_json::json!({"type": "tool_call", "tool": "shell", "duration_ms": 42, "success": true}),
        ),
        (
            ObserverEvent::ToolCallStart { tool: "read".to_string() },
            serde_json::json!({"type": "tool_call_start", "tool": "read"}),
        ),
        (
            ObserverEvent::Error {
                component: "gateway".to_string(),
                message: "failed".to_string(),
            },
            serde_json::json!({"type": "error", "component": "gateway", "message": "failed"}),
        ),
        (
            ObserverEvent::AgentStart {
                provider: "anthropic".to_string(),
                model: "claude".to_string(),
            },
            serde_json::json!({"type": "agent_start", "provider": "anthropic", "model": "claude"}),
        ),
        (
            ObserverEvent::AgentEnd {
                provider: "local".to_string(),
                model: "llama".to_string(),
                duration: Duration::from_millis(7),
                tokens_used: Some(11),
                cost_usd: Some(0.25),
            },
            serde_json::json!({
                "type": "agent_end",
                "provider": "local",
                "model": "llama",
                "duration_ms": 7,
                "tokens_used": 11,
                "cost_usd": 0.25
            }),
        ),
    ];

    for (event, expected) in cases {
        observer.record_event(&event);
        let actual = receiver.try_recv().expect("supported event should broadcast");
        for (key, expected_value) in expected.as_object().expect("expected object") {
            assert_eq!(actual.get(key), Some(expected_value));
        }
        let timestamp = actual
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .expect("supported event should include a timestamp");
        assert!(chrono::DateTime::parse_from_rfc3339(timestamp).is_ok());
    }

    assert_eq!(inner_state.lock().events, 6);
}

#[test]
fn broadcast_observer_serializes_missing_agent_end_statistics_as_null() {
    let (sender, mut receiver) = broadcast::channel(1);
    let observer = BroadcastObserver::new(Box::new(TestObserver::default()), sender);

    observer.record_event(&ObserverEvent::AgentEnd {
        provider: "local".to_string(),
        model: "llama".to_string(),
        duration: Duration::ZERO,
        tokens_used: None,
        cost_usd: None,
    });

    let actual = receiver.try_recv().expect("agent end event should broadcast");
    assert_eq!(actual.get("duration_ms"), Some(&serde_json::json!(0)));
    assert_eq!(actual.get("tokens_used"), Some(&serde_json::Value::Null));
    assert_eq!(actual.get("cost_usd"), Some(&serde_json::Value::Null));
}

#[test]
fn broadcast_observer_forwards_supported_event_when_broadcast_has_no_receivers() {
    let (sender, receiver) = broadcast::channel(1);
    drop(receiver);
    let inner = TestObserver::default();
    let inner_state = inner.state.clone();
    let observer = BroadcastObserver::new(Box::new(inner), sender);

    observer.record_event(&ObserverEvent::AgentStart {
        provider: "local".to_string(),
        model: "llama".to_string(),
    });

    assert_eq!(inner_state.lock().events, 1);
}

#[test]
fn broadcast_observer_ignores_unsupported_events_and_forwards_metrics_and_flush() {
    let (sender, mut receiver) = broadcast::channel(4);
    let inner = TestObserver::default();
    let inner_state = inner.state.clone();
    let observer = BroadcastObserver::new(Box::new(inner), sender);

    observer.record_event(&ObserverEvent::TurnComplete);
    observer.record_metric(&ObserverMetric::QueueDepth(2));
    observer.flush();

    assert!(receiver.try_recv().is_err());
    let state = inner_state.lock();
    assert_eq!(state.events, 1);
    assert_eq!(state.metrics, 1);
    assert_eq!(state.flushes, 1);
}
