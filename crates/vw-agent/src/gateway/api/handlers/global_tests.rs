// 覆盖全局配置、健康状态、事件流及租户隔离接口。

use super::*;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::gateway_skey_store::GatewaySkeyStore;
use crate::app::agent::security::pairing::PairingGuard;
use crate::global_database::GlobalDatabase;
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use parking_lot::Mutex;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio_stream::StreamExt;
use tower::ServiceExt;
use vw_api_types::id::TenantScope;

const BOOTSTRAP_SOURCE: &str = include_str!("../../bootstrap.rs");

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
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

fn state(config: config::Config, require_pairing: bool, tokens: &[String]) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, tokens)),
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
    }
}

fn use_temp_config_dir(temp: &TempDir) -> (EnvGuard, EnvGuard, EnvGuard) {
    let home = EnvGuard::set("HOME", temp.path().as_os_str());
    let config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    (home, config_dir, workspace)
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    serde_json::from_slice(&bytes).expect("json response")
}

async fn patch_config_route(
    app: &Router,
    tenant: &TenantScope,
    patch: Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/global/config")
                .header(vw_api_types::id::PLATFORM_ID_HEADER, tenant.platform_id().as_ref())
                .header(vw_api_types::id::COMPANY_ID_HEADER, tenant.company_id().as_ref())
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(patch.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

fn tenant_request(uri: &str, tenant: &TenantScope) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(vw_api_types::id::PLATFORM_ID_HEADER, tenant.platform_id().as_ref())
        .header(vw_api_types::id::COMPANY_ID_HEADER, tenant.company_id().as_ref())
        .body(Body::empty())
        .expect("request")
}

fn default_tenant_scope() -> axum::extract::Extension<TenantScope> {
    axum::extract::Extension(TenantScope::default())
}

#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
    let _ = stateful_router();
}

#[tokio::test]
async fn global_config_routes_isolate_platform_and_company_updates() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);
    let app = router::<()>()
        .layer(axum::middleware::from_fn(
            crate::app::agent::gateway::middleware::tenant_scope_middleware,
        ))
        .with_state(());
    let first_tenant = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second_tenant = TenantScope::new("platform-b", "company-b").expect("second tenant");

    let second_initial = app
        .clone()
        .oneshot(tenant_request("/global/config", &second_tenant))
        .await
        .expect("response");
    assert_eq!(second_initial.status(), StatusCode::OK);

    let patched = patch_config_route(
        &app,
        &first_tenant,
        serde_json::json!({"default_provider": "tenant-a-provider"}),
    )
    .await;
    assert_eq!(patched.status(), StatusCode::OK);

    let first = app
        .clone()
        .oneshot(tenant_request("/global/config", &first_tenant))
        .await
        .expect("response");
    let second =
        app.oneshot(tenant_request("/global/config", &second_tenant)).await.expect("response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(response_json(first).await["default_provider"], "tenant-a-provider");
    assert_ne!(response_json(second).await["default_provider"], "tenant-a-provider");

    let store = config::SystemConfigStore::new(
        GlobalDatabase::for_home(temp.path()).expect("global database"),
    )
    .expect("system config store");
    let first_payload = store.get(&first_tenant).expect("first payload").expect("first tenant row");
    let second_payload =
        store.get(&second_tenant).expect("second payload").expect("second tenant row");
    assert_eq!(
        serde_json::from_str::<Value>(&first_payload).expect("first json")["default_provider"],
        "tenant-a-provider"
    );
    assert_ne!(
        serde_json::from_str::<Value>(&second_payload).expect("second json")["default_provider"],
        "tenant-a-provider"
    );
}

#[tokio::test]
async fn local_global_config_route_writes_shared_database_only() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);
    let app = router::<()>()
        .layer(axum::middleware::from_fn(
            crate::app::agent::gateway::middleware::tenant_scope_middleware,
        ))
        .with_state(());
    let tenant = TenantScope::default();

    let response = patch_config_route(
        &app,
        &tenant,
        serde_json::json!({
            "default_provider": "local-database-provider",
            "api_key": "local-plain-secret"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let database = GlobalDatabase::for_home(temp.path()).expect("global database");
    assert!(database.path().is_file());
    let store = config::SystemConfigStore::new(database).expect("system config store");
    let payload =
        store.get(&TenantScope::default()).expect("local payload").expect("local tenant row");
    assert_eq!(
        serde_json::from_str::<Value>(&payload).expect("local json")["default_provider"],
        "local-database-provider"
    );
    let get_response =
        app.oneshot(tenant_request("/global/config", &tenant)).await.expect("response");
    assert_eq!(get_response.status(), StatusCode::OK);
    assert_ne!(response_json(get_response).await["api_key"], "local-plain-secret");
    assert!(!temp.path().join("vibewindow.json").exists());
    assert!(!vw_config_types::paths::home_config_dir(temp.path()).join("vibewindow.json").exists());
}

#[tokio::test]
async fn global_config_api_projects_shared_skeys_and_ignores_conflicting_legacy_sqlite() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let store =
        GatewaySkeyStore::new(GlobalDatabase::for_home(temp.path()).expect("global database"))
            .expect("gateway skey store");
    store
        .replace_all(
            &TenantScope::default(),
            &[vw_config_types::gateway::GatewaySkey {
                enabled: true,
                skey: None,
                skey_hash: "a".repeat(64),
                masked_skey: "shared-mask".to_string(),
                name: "shared-only".to_string(),
                expires_at: None,
            }],
        )
        .expect("seed shared skey");

    let legacy_path =
        vw_config_types::paths::home_config_dir(temp.path()).join("gateway").join("skeys.sqlite");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("create legacy parent");
    let legacy = rusqlite::Connection::open(&legacy_path).expect("legacy sqlite");
    legacy
        .execute_batch(
            r#"
            CREATE TABLE gateway_skeys (
                skey_hash TEXT PRIMARY KEY NOT NULL,
                masked_skey TEXT NOT NULL,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT INTO gateway_skeys (skey_hash, masked_skey, name, enabled)
            VALUES ('bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
                    'legacy-mask', 'legacy-conflict', 1);
            "#,
        )
        .expect("seed legacy sqlite");
    drop(legacy);

    let Json(value) = global_config_get(default_tenant_scope()).await.expect("global config");

    assert_eq!(value["gateway"]["skeys"][0]["name"], "shared-only");
    assert!(!value.to_string().contains("legacy-conflict"));
    assert!(legacy_path.exists(), "legacy sqlite must remain untouched");
}

#[test]
fn gateway_bootstrap_loads_default_tenant_skeys_from_shared_store_without_legacy_fallback() {
    assert!(BOOTSTRAP_SOURCE.contains("load_gateway_skeys_for_tenant"));
    assert!(BOOTSTRAP_SOURCE.contains("TenantScope::default()"));
    assert!(!BOOTSTRAP_SOURCE.contains("load_existing_gateway_skeys"));
    assert!(!BOOTSTRAP_SOURCE.contains("falling back to config"));
}

#[tokio::test]
async fn global_health_reports_ok_status() {
    let Json(value) = global_health().await;

    assert_eq!(value.get("status"), Some(&serde_json::Value::String("ok".to_string())));
    assert_eq!(value.get("healthy"), Some(&serde_json::Value::Bool(true)));
    assert!(value.get("version").is_some());
    assert!(value.get("health").is_some());
}

#[tokio::test]
async fn global_config_get_serializes_current_config() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);

    let Json(value) = global_config_get(default_tenant_scope()).await.expect("global config");

    assert!(value.is_object());
    assert!(value.get("default_provider").is_some());
}

#[tokio::test]
async fn global_config_get_maps_unusable_config_directory_to_bad_request() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let unusable_config_dir = temp.path().join("config-file");
    std::fs::write(&unusable_config_dir, "not a directory").expect("config sentinel");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", unusable_config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let error = global_config_get(default_tenant_scope())
        .await
        .expect_err("a file cannot be used as the config directory");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.message.contains("Failed to create config directory"));
}

#[test]
fn config_response_maps_secret_encryption_failure_to_internal_error() {
    let mut config = config::Config::default();
    config.config_path = std::path::PathBuf::new();

    let err = config_response(config).expect_err("missing config parent should fail");

    assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(err.to_string().contains("Config path must have a parent directory"));
}

#[tokio::test]
async fn global_config_acp_get_returns_merged_agent_specs() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);

    let Json(value) = global_config_acp_get().await;

    let specs = value.as_object().expect("acp config should serialize as an object");
    assert!(!specs.is_empty());
    assert!(specs.values().all(|spec| spec.get("command").is_some()));
}

#[tokio::test]
async fn global_config_acp_get_trims_and_filters_user_overrides() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);

    let Json(body) = global_config_patch(
        default_tenant_scope(),
        Json(serde_json::json!({
            "acp": {
                "  Custom  ": {
                    "command": " custom-bin ",
                    "args": ["--stdio"],
                    "env": {"KEY": "VALUE"}
                },
                "blank": {
                    "command": "   "
                }
            }
        })),
    )
    .await
    .expect("acp patch should succeed");

    assert_eq!(body, serde_json::Value::Null);

    let Json(value) = global_config_acp_get().await;

    assert_eq!(value["custom"]["command"], "custom-bin");
    assert_eq!(value["custom"]["args"], serde_json::json!(["--stdio"]));
    assert_eq!(value["custom"]["env"], serde_json::json!({"KEY": "VALUE"}));
    assert!(value.get("blank").is_none());
}

#[tokio::test]
async fn global_config_patch_accepts_object_patch() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);

    let Json(body) = global_config_patch(
        default_tenant_scope(),
        Json(serde_json::json!({
            "default_provider": "global-test-provider"
        })),
    )
    .await
    .expect("object patch should be accepted");
    let Json(next) = global_config_get(default_tenant_scope()).await.expect("global config");

    assert_eq!(body, serde_json::Value::Null);
    assert_eq!(next["default_provider"], "global-test-provider");
}

#[tokio::test]
async fn global_config_patch_preserves_mcp_servers() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);

    let Json(body) = global_config_patch(
        default_tenant_scope(),
        Json(serde_json::json!({
            "mcpServers": [{
                "name": "local",
                "type": "stdio",
                "command": "npx",
                "args": ["-y", "demo"]
            }]
        })),
    )
    .await
    .expect("mcp patch should be accepted");

    let Json(next) = global_config_get(default_tenant_scope()).await.expect("global config");

    assert_eq!(body, serde_json::Value::Null);
    assert_eq!(next["mcpServers"][0]["name"], "local");
    assert_eq!(next["mcpServers"][0]["args"], serde_json::json!(["-y", "demo"]));
}

#[tokio::test]
async fn global_config_patch_rejects_non_object_patch() {
    let err = global_config_patch(default_tenant_scope(), Json(serde_json::json!("bad-patch")))
        .await
        .expect_err("non-object patch should be rejected");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert!(err.to_string().contains("config patch must be a JSON object"));
}

#[tokio::test]
async fn global_config_patch_stateful_updates_cached_config_and_pairing() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);
    let state = state(config::Config::default(), false, &[]);

    let Json(body) = global_config_patch_stateful(
        default_tenant_scope(),
        State(state.clone()),
        Json(serde_json::json!({
            "default_provider": "stateful-provider",
            "gateway": {
                "auth_enabled": true,
                "skeys": [{
                    "enabled": true,
                    "skey": "stateful-secret",
                    "name": "ci"
                }]
            }
        })),
    )
    .await
    .expect("stateful patch should succeed");

    assert_eq!(body, serde_json::Value::Null);
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("stateful-provider"));
    assert!(state.config.lock().gateway.auth_enabled);
    assert!(state.pairing.auth_enabled());
    assert_eq!(state.pairing.active_skey_count(), 1);
    assert!(state.pairing.is_authenticated("stateful-secret"));
    assert!(!state.pairing.is_authenticated("wrong-secret"));
}

#[tokio::test]
async fn global_config_patch_stateful_rejects_invalid_patch_without_runtime_update() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);
    let state = state(config::Config::default(), false, &[]);
    let initial_default_provider = state.config.lock().default_provider.clone();

    let err = global_config_patch_stateful(
        default_tenant_scope(),
        State(state.clone()),
        Json(serde_json::json!("bad")),
    )
    .await
    .expect_err("invalid stateful patch should fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(state.config.lock().default_provider, initial_default_provider);
    assert!(!state.pairing.auth_enabled());
}

#[tokio::test]
async fn global_dispose_returns_true_and_publishes_global_event() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let unsub = bus::global_subscribe(move |event| {
        if event.payload.get("type").and_then(Value::as_str) == Some("global.disposed") {
            let _ = tx.send(event);
        }
    });

    let Json(disposed) = global_dispose().await.expect("dispose should succeed");
    let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("dispose event should arrive")
        .expect("dispose event should be sent");
    unsub();

    assert!(disposed);
    assert_eq!(event.directory.as_deref(), Some("global"));
    assert_eq!(event.payload["type"], "global.disposed");
    assert_eq!(event.payload["properties"], serde_json::json!({}));
}

#[tokio::test(start_paused = true)]
async fn global_event_sse_response_uses_event_stream_content_type() {
    let response = global_event_sse().await.into_response();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response.headers().get(axum::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );

    drop(response);
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::task::yield_now().await;
}

#[tokio::test]
async fn global_event_sse_streams_connected_and_published_events() {
    tokio::time::pause();
    let response = global_event_sse().await.into_response();
    let mut stream = response.into_body().into_data_stream();

    let connected = stream
        .next()
        .await
        .expect("connected event should be present")
        .expect("connected event frame should be readable");
    let connected = String::from_utf8(connected.to_vec()).expect("connected event should be utf8");

    bus::publish(bus::define("test.global.sse"), serde_json::json!({"ok": true}), None)
        .expect("publish should succeed");

    let mut published_chunks = Vec::new();
    for _ in 0..3 {
        let chunk = stream
            .next()
            .await
            .expect("published event should be present")
            .expect("published event frame should be readable");
        let chunk = String::from_utf8(chunk.to_vec()).expect("published event should be utf8");
        let found = chunk.contains("\"type\":\"test.global.sse\"");
        published_chunks.push(chunk);
        if found {
            break;
        }
    }
    let published = published_chunks.join("");
    drop(stream);
    for _ in 0..2 {
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::task::yield_now().await;
    }

    assert!(connected.contains("\"type\":\"server.connected\""));
    assert!(published.contains("\"directory\":\"global\""));
    assert!(published.contains("\"type\":\"test.global.sse\""));
    assert!(published.contains("\"ok\":true"));
}

#[tokio::test]
async fn global_event_sse_preserves_published_event_directory() {
    tokio::time::pause();
    let response = global_event_sse().await.into_response();
    let mut stream = response.into_body().into_data_stream();
    let _connected = stream
        .next()
        .await
        .expect("connected event should be present")
        .expect("connected event frame should be readable");

    bus::publish(
        bus::define("test.global.sse.directory"),
        serde_json::json!({"ok": true}),
        Some("workspace-a".to_string()),
    )
    .expect("publish should succeed");

    let mut published = String::new();
    for _ in 0..3 {
        let chunk = stream
            .next()
            .await
            .expect("published event should be present")
            .expect("published event frame should be readable");
        published.push_str(&String::from_utf8(chunk.to_vec()).expect("event should be utf8"));
        if published.contains("\"type\":\"test.global.sse.directory\"") {
            break;
        }
    }

    assert!(published.contains("\"directory\":\"workspace-a\""));
    assert!(published.contains("\"type\":\"test.global.sse.directory\""));

    drop(stream);
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::task::yield_now().await;
}

#[tokio::test(start_paused = true)]
async fn global_event_sse_streams_heartbeat_and_stops_after_receiver_closes() {
    let response = global_event_sse().await.into_response();
    let mut stream = response.into_body().into_data_stream();
    let mut frames = String::new();

    for _ in 0..6 {
        let chunk = stream
            .next()
            .await
            .expect("initial SSE event should be present")
            .expect("initial SSE event frame should be readable");
        frames.push_str(&String::from_utf8(chunk.to_vec()).expect("event should be utf8"));
        if frames.contains("\"type\":\"server.heartbeat\"") {
            break;
        }
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::task::yield_now().await;
    }

    assert!(frames.contains("\"type\":\"server.connected\""));
    assert!(frames.contains("\"type\":\"server.heartbeat\""));

    drop(stream);
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::task::yield_now().await;
}
