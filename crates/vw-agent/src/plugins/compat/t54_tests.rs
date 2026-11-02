use crate::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::{
    config::Config, memory::NoneMemory, providers::Provider, security::pairing::PairingGuard,
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
#[tokio::test]
async fn codex_plugin_compat_t54_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let app = app(root.path());
    assert_eq!(
        post(&app, "/plugins/prepare-install", json!({"source":"npm:../bad"}), true).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post(&app, "/plugins/update", json!({"token":"unknown","approved":true}), true).await.0,
        StatusCode::BAD_REQUEST
    );
    assert!(!root.path().join("config/plugins").exists());
}
#[tokio::test]
async fn codex_plugin_compat_t54_preserves_isolation() {
    let first = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let source = first.path().join("source");
    package(&source, "1");
    let a = app(first.path());
    let b = app(other.path());
    let (status, preview) =
        post(&a, "/plugins/prepare-install", json!({"source":source}), true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        post(&b, "/plugins/install", json!({"token":preview["token"],"approved":true}), true)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        post(&a, "/plugins/install", json!({"token":preview["token"],"approved":true}), true)
            .await
            .0,
        StatusCode::OK
    );
    package(&source, "2");
    let (status, preview) =
        post(&a, "/plugins/prepare-update", json!({"plugin_id":"fixture","source":source}), true)
            .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["old_version"], "1");
    assert_eq!(preview["version"], "2");
    assert_eq!(
        post(&a, "/plugins/update", json!({"token":preview["token"],"approved":true}), true)
            .await
            .0,
        StatusCode::OK
    );
    let installed = first.path().join("config/plugins/fixture");
    assert_eq!(
        crate::plugins::load_plugin_manifest(&installed).unwrap().version.as_deref(),
        Some("2")
    );
    assert!(!other.path().join("config/plugins").exists());
}

#[tokio::test]
async fn codex_plugin_compat_t54_gateway_client_activates_audited_package() {
    use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    package(&source, "1");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new().nest("/v1", app(root.path()));
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", address.port())
            .with_auth(GatewayAuth { skey: Some("fixture-token".into()) }),
    )
    .unwrap();
    let preview = client.plugin_prepare_install(source.to_str().unwrap()).await.unwrap();
    assert_eq!(preview.origin.kind, "local");
    assert_eq!(preview.origin.resolved_version.as_deref(), Some("1"));
    assert_eq!(preview.dependency_status, "unchecked");
    let path = client.plugin_install(&preview.token, true).await.unwrap();
    assert!(Path::new(&path).is_dir());
    assert!(client.plugin_install(&preview.token, true).await.is_err());
    let cancelled = client.plugin_prepare_install(source.to_str().unwrap()).await.unwrap();
    client.plugin_cancel(&cancelled.token).await.unwrap();
    assert!(client.plugin_install(&cancelled.token, true).await.is_err());
    server.abort();
    let _ = server.await;
}
