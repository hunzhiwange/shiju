//! Gateway metrics 端点的访问控制与输出格式测试。
//!
//! 这些测试直接调用健康检查处理器，覆盖 Prometheus 未启用时的提示、启用后
//! 的指标渲染，以及 pairing/公网访问约束，防止 metrics 暴露面被意外放宽。

use super::health::PROMETHEUS_CONTENT_TYPE;
use super::*;
use axum::body::to_bytes;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use reqwest::header::{self, HeaderMap};

#[tokio::test]
async fn metrics_endpoint_returns_hint_when_prometheus_is_disabled() {
    // 使用 NoopObserver 时仍返回 Prometheus 文本类型，客户端无需为禁用状态换解析器。
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(MockProvider::default()),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(MockMemory),
        auto_save: false,
        webhook_secret_hash: None,
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    let response: Response = crate::app::agent::gateway::health::handle_metrics(
        State(state),
        test_connect_info(),
        HeaderMap::new(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value: &axum::http::HeaderValue| value.to_str().ok()),
        Some(PROMETHEUS_CONTENT_TYPE)
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("Prometheus backend not enabled"));
}

#[tokio::test]
async fn metrics_endpoint_renders_prometheus_output() {
    let prom = Arc::new(crate::app::agent::observability::PrometheusObserver::new());
    // 先记录一个确定事件，避免测试依赖后台心跳的时间窗口。
    crate::app::agent::observability::Observer::record_event(
        prom.as_ref(),
        &crate::app::agent::observability::ObserverEvent::HeartbeatTick,
    );

    let observer: Arc<dyn crate::app::agent::observability::Observer> = prom;
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(MockProvider::default()),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(MockMemory),
        auto_save: false,
        webhook_secret_hash: None,
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
        observer,
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    let response: Response = crate::app::agent::gateway::health::handle_metrics(
        State(state),
        test_connect_info(),
        HeaderMap::new(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("vibewindow_heartbeat_ticks_total 1"));
}

#[tokio::test]
async fn metrics_endpoint_rejects_public_clients_when_pairing_is_disabled() {
    // metrics 可能泄露运行状态；未启用 pairing 时只允许本地回环访问。
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(MockProvider::default()),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(MockMemory),
        auto_save: false,
        webhook_secret_hash: None,
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    let response: Response = crate::app::agent::gateway::health::handle_metrics(
        State(state),
        test_public_connect_info(),
        HeaderMap::new(),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("non-loopback"));
}

#[tokio::test]
async fn metrics_endpoint_requires_bearer_token_when_pairing_is_enabled() {
    let paired_token = "zc_test_token".to_string();
    // pairing 开启后，即使是本地访问也必须带已配对 token。
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(MockProvider::default()),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(MockMemory),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(true, std::slice::from_ref(&paired_token))),
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    let unauthorized: Response = crate::app::agent::gateway::health::handle_metrics(
        State(state.clone()),
        test_connect_info(),
        HeaderMap::new(),
    )
    .await
    .into_response();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    // 只验证 Bearer token 通过路径，不把 token 打印到失败信息里。
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {paired_token}")).unwrap(),
    );
    let authorized: Response = crate::app::agent::gateway::health::handle_metrics(
        State(state),
        test_connect_info(),
        headers,
    )
    .await
    .into_response();
    assert_eq!(authorized.status(), StatusCode::OK);
}
