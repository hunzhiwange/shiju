use model_types::Provider;
use crate::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::{
    config::Config, memory::NoneMemory, security::pairing::PairingGuard,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};
use tower::ServiceExt;
struct StaticProvider;
#[async_trait::async_trait]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        Ok("unused".into())
    }
}
fn app(root: &Path) -> Router {
    let mut config = Config::default();
    config.config_path = root.join("config/config.json");
    config.workspace_dir = root.join("workspace");
    let state = AppState {
        config: Arc::new(Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "fixture".into(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(true, &["fixture-token".into()])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 100)),
        observer: Arc::new(crate::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: Default::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    };
    crate::gateway::api::handlers::plugins::stateful_router().with_state(state)
}
async fn post(app: &Router, path: &str, body: Value, authorized: bool) -> (StatusCode, Value) {
    let mut request = Request::post(path).header("content-type", "application/json");
    if authorized {
        request = request.header("authorization", "Bearer fixture-token");
    }
    let response =
        app.clone().oneshot(request.body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
fn package(root: &Path, version: &str) {
    std::fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    std::fs::write(
        root.join(".codex-plugin/plugin.json"),
        format!(r#"{{"name":"fixture","version":"{version}"}}"#),
    )
    .unwrap();
}
#[tokio::test]
async fn codex_plugin_compat_t54_requires_authorized_install_request() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    package(&source, "1");
    let app = app(root.path());
    assert_eq!(
        post(&app, "/plugins/prepare-install", json!({"source":source}), false).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, preview) =
        post(&app, "/plugins/prepare-install", json!({"source":source}), true).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert!(!root.path().join("config/plugins/fixture").exists());
    assert_eq!(
        post(&app, "/plugins/install", json!({"token":preview["token"],"approved":false}), true)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post(&app, "/plugins/install", json!({"token":preview["token"],"approved":true}), false)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&app, "/plugins/install", json!({"token":preview["token"],"approved":true}), true)
            .await
            .0,
        StatusCode::OK
    );
    assert!(root.path().join("config/plugins/fixture").exists());
}
