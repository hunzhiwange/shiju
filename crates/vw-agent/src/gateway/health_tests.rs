// 验证健康检查与指标端点的访问限制、内容类型和运行时快照。

// 公网客户端与回环地址分开构造，用于确认指标端点不会因关闭配对而放宽。
use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use parking_lot::Mutex;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

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
        Ok("ok".into())
    }
}

fn state_with_pairing(pairing: PairingGuard) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(pairing),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1_000)),
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
        multimodal: Default::default(),
        max_iterations: 5,
        event_tx: tokio::sync::broadcast::channel(1).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn loopback() -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080))
}

fn public_client() -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)), 8080))
}

async fn body_text(response: Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn health_returns_status_auth_flags_and_runtime_snapshot() {
    crate::app::agent::health::mark_component_ok("gateway-health-test");
    let build_id = crate::app::agent::health::initialize_runtime_build_id().unwrap();
    let state = state_with_pairing(PairingGuard::new(true, &[]));

    let response = handle_health(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["build_id"], build_id);
    assert_eq!(json["auth_enabled"], true);
    assert_eq!(json["active_skeys"], 0);
    assert!(json["runtime"]["pid"].as_u64().is_some());
    assert_eq!(json["runtime"]["components"]["gateway-health-test"]["status"], "ok");
}

#[tokio::test]
async fn metrics_returns_hint_with_prometheus_content_type_for_loopback_noop_observer() {
    let response = handle_metrics(
        State(state_with_pairing(PairingGuard::new(false, &[]))),
        loopback(),
        HeaderMap::new(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()),
        Some(PROMETHEUS_CONTENT_TYPE)
    );
    assert!(body_text(response).await.contains("Prometheus backend not enabled"));
}

#[tokio::test]
async fn metrics_rejects_public_clients_when_skey_auth_is_disabled() {
    let response = handle_metrics(
        State(state_with_pairing(PairingGuard::new(false, &[]))),
        public_client(),
        HeaderMap::new(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(body_text(response).await.contains("non-loopback"));
}

#[tokio::test]
async fn metrics_requires_valid_bearer_skey_when_auth_is_enabled() {
    let token = "paired-token".to_string();
    let state = state_with_pairing(PairingGuard::new(true, std::slice::from_ref(&token)));

    let unauthorized =
        handle_metrics(State(state.clone()), loopback(), HeaderMap::new()).await.into_response();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    assert!(body_text(unauthorized).await.contains("unauthorized"));

    let mut invalid_scheme = HeaderMap::new();
    invalid_scheme.insert(header::AUTHORIZATION, HeaderValue::from_static("Basic paired-token"));
    let unauthorized =
        handle_metrics(State(state.clone()), loopback(), invalid_scheme).await.into_response();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let mut headers = HeaderMap::new();
    headers
        .insert(header::AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {token}")).unwrap());
    let authorized = handle_metrics(State(state), loopback(), headers).await.into_response();
    assert_eq!(authorized.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_renders_prometheus_backend_when_enabled() {
    let prom = Arc::new(crate::app::agent::observability::PrometheusObserver::new());
    crate::app::agent::observability::Observer::record_event(
        prom.as_ref(),
        &crate::app::agent::observability::ObserverEvent::HeartbeatTick,
    );

    let mut state = state_with_pairing(PairingGuard::new(false, &[]));
    state.observer = prom;

    let response = handle_metrics(State(state), loopback(), HeaderMap::new()).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_text(response).await.contains("vibewindow_heartbeat_ticks_total 1"));
}
