use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Method, Request, StatusCode, header};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower::ServiceExt;
use vw_api_types::id::TenantScope;

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
        Ok("{}".to_string())
    }
}

fn state() -> AppState {
    let (event_tx, _) = broadcast::channel(16);

    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
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
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn test_app() -> Router {
    router().layer(Extension(TenantScope::default())).with_state(state())
}

async fn response_status(method: Method, uri: &str, body: Option<&str>) -> StatusCode {
    let mut request = Request::builder().method(method).uri(uri);
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    test_app()
        .oneshot(
            request
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[test]
fn data_router_builds_all_routes() {
    let _router: Router<AppState> = router();
}

#[tokio::test]
async fn data_router_serves_read_routes_and_unknown_path() {
    assert_eq!(response_status(Method::GET, "/data/settings", None).await, StatusCode::OK);
    assert_eq!(response_status(Method::GET, "/data/connections", None).await, StatusCode::OK);
    assert_eq!(response_status(Method::GET, "/data/reports", None).await, StatusCode::OK);
    assert_eq!(response_status(Method::GET, "/data/not-found", None).await, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn data_router_matches_write_and_parameterized_routes() {
    let cases = [
        (Method::PUT, "/data/settings", Some("{}"), StatusCode::UNPROCESSABLE_ENTITY),
        (Method::POST, "/data/connections", Some("{}"), StatusCode::UNPROCESSABLE_ENTITY),
        (Method::GET, "/data/connections/missing", None, StatusCode::NOT_FOUND),
        (Method::PUT, "/data/connections/missing", Some("{}"), StatusCode::UNPROCESSABLE_ENTITY),
        (Method::DELETE, "/data/connections/missing", None, StatusCode::NOT_FOUND),
        (Method::POST, "/data/connections/missing/activate", None, StatusCode::NOT_FOUND),
        (Method::POST, "/data/connections/missing/test", None, StatusCode::NOT_FOUND),
        (Method::GET, "/data/connections/missing/catalog", None, StatusCode::NOT_FOUND),
        (Method::POST, "/data/reports", Some("{}"), StatusCode::UNPROCESSABLE_ENTITY),
        (Method::GET, "/data/reports/missing", None, StatusCode::NOT_FOUND),
        (Method::PUT, "/data/reports/missing", Some("{}"), StatusCode::UNPROCESSABLE_ENTITY),
        (Method::DELETE, "/data/reports/missing", None, StatusCode::NOT_FOUND),
        (Method::POST, "/data/query", Some("{}"), StatusCode::BAD_REQUEST),
        (Method::POST, "/data/ai/query", Some("{}"), StatusCode::UNPROCESSABLE_ENTITY),
    ];

    for (method, uri, body, expected) in cases {
        assert_eq!(response_status(method, uri, body).await, expected, "{uri}");
    }
}

#[tokio::test]
async fn data_router_rejects_unsupported_methods() {
    assert_eq!(
        response_status(Method::POST, "/data/settings", Some("{}")).await,
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        response_status(Method::GET, "/data/query", None).await,
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn data_router_rejects_near_miss_paths() {
    let cases = [
        (Method::GET, "/data/settings/"),
        (Method::GET, "/data/connections/"),
        (Method::GET, "/data/connections//catalog"),
        (Method::POST, "/data/connections/missing/activate/extra"),
        (Method::GET, "/data/reports/missing/extra"),
        (Method::POST, "/data/query/"),
    ];

    for (method, uri) in cases {
        assert_eq!(response_status(method, uri, None).await, StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn data_router_rejects_json_routes_without_json_content_type() {
    let cases = [
        (Method::PUT, "/data/settings"),
        (Method::POST, "/data/connections"),
        (Method::PUT, "/data/connections/missing"),
        (Method::POST, "/data/reports"),
        (Method::PUT, "/data/reports/missing"),
        (Method::POST, "/data/query"),
        (Method::POST, "/data/ai/query"),
    ];

    for (method, uri) in cases {
        let response = test_app()
            .oneshot(Request::builder().method(method).uri(uri).body(Body::from("{}")).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE, "{uri}");
    }
}

#[tokio::test]
async fn data_router_rejects_malformed_json_before_handler_execution() {
    let cases = [
        (Method::PUT, "/data/settings"),
        (Method::POST, "/data/connections"),
        (Method::POST, "/data/reports"),
        (Method::POST, "/data/query"),
        (Method::POST, "/data/ai/query"),
    ];

    for (method, uri) in cases {
        assert_eq!(response_status(method, uri, Some("{")).await, StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[tokio::test]
async fn data_router_rejects_unsupported_methods_on_nested_routes() {
    let cases = [
        (Method::PATCH, "/data/connections/missing"),
        (Method::DELETE, "/data/connections/missing/activate"),
        (Method::PUT, "/data/connections/missing/test"),
        (Method::POST, "/data/connections/missing/catalog"),
        (Method::PATCH, "/data/reports/missing"),
        (Method::GET, "/data/ai/query"),
    ];

    for (method, uri) in cases {
        assert_eq!(
            response_status(method, uri, None).await,
            StatusCode::METHOD_NOT_ALLOWED,
            "{uri}"
        );
    }
}
