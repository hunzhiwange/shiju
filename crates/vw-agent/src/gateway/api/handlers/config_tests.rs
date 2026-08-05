use super::*;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::extract::{Extension, Query};
use axum::http::{HeaderValue, header};
use parking_lot::Mutex;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use vw_api_types::id::TenantScope;

use crate::global_database::GlobalDatabase;

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
        max_tool_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
    }
}

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {token}");
    headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&value).unwrap());
    headers
}

fn instance_query(temp: &TempDir) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) })
}

fn directory_query(directory: &std::path::Path) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) })
}

fn local_store(home: &std::path::Path) -> config::SystemConfigStore {
    config::SystemConfigStore::new(GlobalDatabase::for_home(home).expect("global database"))
        .expect("system config store")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be json")
}

#[test]
fn routers_build_for_stateless_and_stateful_config_routes() {
    let _: axum::Router<()> = router();
    let _ = stateful_router();
}

#[test]
fn has_explicit_directory_detects_non_empty_query_or_header_values() {
    let mut headers = HeaderMap::new();
    let empty_query = InstanceQuery { directory: None };
    assert!(!has_explicit_directory(&empty_query, &headers));

    let blank_query = InstanceQuery { directory: Some("  ".to_string()) };
    assert!(!has_explicit_directory(&blank_query, &headers));

    let query = InstanceQuery { directory: Some("/tmp/project".to_string()) };
    assert!(has_explicit_directory(&query, &headers));

    headers.insert("x-vibewindow-directory", HeaderValue::from_static("  "));
    assert!(!has_explicit_directory(&empty_query, &headers));

    headers.insert("x-vibewindow-directory", HeaderValue::from_static("/tmp/project"));
    assert!(has_explicit_directory(&empty_query, &headers));
}

#[test]
fn has_explicit_directory_ignores_non_utf8_header_value() {
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    let invalid = HeaderValue::from_bytes(&[0xff]).expect("non-utf8 header bytes");
    headers.insert("x-vibewindow-directory", invalid);

    assert!(!has_explicit_directory(&query, &headers));
}

#[tokio::test]
async fn config_get_returns_serialized_instance_config() {
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(body) = config_get(
        Extension(TenantScope::default()),
        instance_query(&temp),
        HeaderMap::new(),
    )
    .await
    .expect("config should load");

    assert!(body.is_object());
    assert!(body.get("default_provider").is_some());
}

#[tokio::test]
async fn config_get_stateful_returns_serialized_instance_config() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(body) = config_get_stateful(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: Some(home.path().to_string_lossy().to_string()) }),
        HeaderMap::new(),
    )
    .await
    .expect("stateful config should load");

    assert!(body.is_object());
    assert!(body.get("default_provider").is_some());
}

#[tokio::test]
async fn config_get_without_directory_restores_the_requested_tenant_after_restart() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::new("desktop-platform", "desktop-company").expect("tenant");

    config::update_global_for_tenant(
        &tenant,
        serde_json::json!({"agents": {"main": {"max_iterations": 123}}}),
    )
    .await
    .expect("tenant config should persist");

    let Json(body) = config_get(
        Extension(tenant),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("tenant config should reload");

    assert_eq!(body["agents"]["main"]["max_iterations"], 123);
    assert_ne!(
        config::get().await.agents["main"].max_iterations,
        123,
        "local gateway startup config must not replace the requested tenant"
    );
}

#[tokio::test]
async fn config_patch_rejects_non_object_patch() {
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let err = config_patch(
        Extension(TenantScope::default()),
        instance_query(&temp),
        HeaderMap::new(),
        Json(serde_json::json!("invalid-patch")),
    )
    .await
    .expect_err("non-object patch should be rejected");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("config patch must be a JSON object"));
}

#[tokio::test]
async fn config_patch_updates_global_config_without_explicit_directory() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(body) = config_patch(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "global-provider"})),
    )
    .await
    .expect("global patch should succeed");

    assert_eq!(body["default_provider"], "global-provider");
    assert_eq!(config::get().await.default_provider.as_deref(), Some("global-provider"));
}

#[tokio::test]
async fn config_patch_without_directory_updates_only_the_requested_tenant() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let tenant = TenantScope::new("desktop-platform", "desktop-company").expect("tenant");

    let Json(body) = config_patch(
        Extension(tenant.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "tenant-provider"})),
    )
    .await
    .expect("tenant patch should succeed");

    assert_eq!(body["default_provider"], "tenant-provider");
    assert_eq!(
        config::get_global_for_tenant(&tenant).await.expect("tenant config").default_provider,
        Some("tenant-provider".to_string())
    );
    assert_ne!(config::get().await.default_provider, Some("tenant-provider".to_string()));
}

#[tokio::test]
async fn config_patch_uses_explicit_header_for_project_override() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project = home.path().join("project");
    std::fs::create_dir(&project).expect("project dir");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-vibewindow-directory",
        HeaderValue::from_str(&project.to_string_lossy()).expect("project header"),
    );

    let Json(body) = config_patch(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: None }),
        headers,
        Json(serde_json::json!({"default_provider": "header-project-provider"})),
    )
    .await
    .expect("header project patch should succeed");

    assert_eq!(body["default_provider"], "header-project-provider");
    let override_value: Value = serde_json::from_str(
        &std::fs::read_to_string(project.join(".vibewindow.json")).expect("project override"),
    )
    .expect("override json");
    assert_eq!(override_value, serde_json::json!({"default_provider": "header-project-provider"}));
}

#[tokio::test]
async fn config_patch_stateful_updates_active_runtime_config_without_explicit_directory() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let state = state(config::Config::default(), false, &[]);

    let Json(body) = config_patch_stateful(
        Extension(TenantScope::default()),
        State(state.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "openai"})),
    )
    .await
    .expect("active config patch should succeed");

    assert_eq!(body["default_provider"], "openai");
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("openai"));
}

#[tokio::test]
async fn config_patch_stateful_leaves_runtime_config_when_directory_is_explicit() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let state = state(config::Config::default(), false, &[]);

    let Json(body) = config_patch_stateful(
        Extension(TenantScope::default()),
        State(state.clone()),
        instance_query(&temp),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "openai"})),
    )
    .await
    .expect("explicit directory config patch should succeed");

    assert_eq!(body["default_provider"], "openai");
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("openrouter"));
}

#[tokio::test]
async fn config_patch_stateful_rejects_invalid_global_patch_without_mutating_runtime_config() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let current = config::Config::default();
    let expected_provider = current.default_provider.clone();
    let state = state(current, false, &[]);

    let err = config_patch_stateful(
        Extension(TenantScope::default()),
        State(state.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(serde_json::json!("invalid-patch")),
    )
    .await
    .expect_err("non-object global patch should be rejected");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("config patch must be a JSON object"));
    assert_eq!(state.config.lock().default_provider, expected_provider);
}

#[tokio::test]
async fn project_config_patch_writes_only_dot_vibewindow_override() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project_one = home.path().join("project-one");
    let project_two = home.path().join("project-two");
    std::fs::create_dir_all(&project_one).expect("first project");
    std::fs::create_dir_all(&project_two).expect("second project");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update_global(serde_json::json!({
        "default_provider": "base-provider",
        "default_model": "base-model"
    }))
    .await
    .expect("seed base config");
    let store = local_store(home.path());
    let base_before = store.get(&TenantScope::default()).unwrap().expect("local base row");

    let Json(response) = config_patch(
        Extension(TenantScope::default()),
        directory_query(&project_one),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "project-provider"})),
    )
    .await
    .expect("project patch");

    assert_eq!(response["default_provider"], "project-provider");
    assert_eq!(response["default_model"], "base-model");
    let override_value: Value = serde_json::from_str(
        &std::fs::read_to_string(project_one.join(".vibewindow.json")).expect("project override"),
    )
    .expect("override json");
    assert_eq!(override_value, serde_json::json!({"default_provider": "project-provider"}));
    assert!(!project_two.join(".vibewindow.json").exists());
    assert_eq!(
        store.get(&TenantScope::default()).unwrap().expect("local base row after patch"),
        base_before
    );
}

#[tokio::test]
async fn project_config_patch_never_persists_effective_base_fields() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let home = TempDir::new().expect("temp home");
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).expect("project");
    let config_dir = home.path().join("config");
    let _home = EnvGuard::set("HOME", home.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    config::update_global(serde_json::json!({
        "default_provider": "base-provider",
        "default_model": "base-model"
    }))
    .await
    .expect("seed base config");
    std::fs::write(
        project.join(".vibewindow.json"),
        serde_json::json!({"default_temperature": 0.4}).to_string(),
    )
    .expect("seed raw override");
    let store = local_store(home.path());
    let base_before = store.get(&TenantScope::default()).unwrap().expect("local base row");

    let Json(_) = config_patch(
        Extension(TenantScope::default()),
        directory_query(&project),
        HeaderMap::new(),
        Json(serde_json::json!({"default_provider": "project-provider"})),
    )
    .await
    .expect("project patch");

    let override_value: Value = serde_json::from_str(
        &std::fs::read_to_string(project.join(".vibewindow.json")).expect("project override"),
    )
    .expect("override json");
    assert_eq!(
        override_value,
        serde_json::json!({
            "default_provider": "project-provider",
            "default_temperature": 0.4
        })
    );
    assert!(override_value.get("default_model").is_none());
    assert_eq!(
        store.get(&TenantScope::default()).unwrap().expect("local base row after patch"),
        base_before
    );
}

#[tokio::test]
async fn config_providers_returns_defaults_for_available_models() {
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");

    let Json(response) = config_providers(instance_query(&temp), HeaderMap::new())
        .await
        .expect("providers should load");

    for provider in response.providers.iter().filter(|provider| !provider.models.is_empty()) {
        assert!(response.default.contains_key(&provider.id));
    }
}

#[tokio::test]
async fn handle_api_config_get_requires_auth_when_pairing_is_enabled() {
    let token = "paired-token".to_string();
    let response = handle_api_config_get(
        State(state(config::Config::default(), true, std::slice::from_ref(&token))),
        Extension(TenantScope::default()),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Unauthorized"));
}

#[tokio::test]
async fn handle_api_config_get_returns_pretty_toml_with_masked_secrets() {
    let mut cfg = config::Config::default();
    cfg.api_key = Some("sk-secret".to_string());
    cfg.gateway.paired_tokens = vec!["paired-secret".to_string()];

    let response = handle_api_config_get(
        State(state(cfg, false, &[])),
        Extension(TenantScope::default()),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["format"], "toml");
    let content = body["content"].as_str().unwrap();
    assert!(content.contains("api_key = \"***MASKED***\""));
    assert!(!content.contains("sk-secret"));
    assert!(!content.contains("paired-secret"));
}

#[tokio::test]
async fn handle_api_config_get_reports_gateway_skey_store_failure() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let invalid_home = temp.path().join("home-file");
    std::fs::write(&invalid_home, "not a directory").expect("invalid home file");
    let _home = EnvGuard::set("HOME", invalid_home.as_os_str());

    let response = handle_api_config_get(
        State(state(config::Config::default(), false, &[])),
        Extension(TenantScope::default()),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("Failed to load gateway skeys")
    );
}

#[tokio::test]
async fn handle_api_config_put_rejects_unauthorized_before_parsing_body() {
    let token = "paired-token".to_string();
    let response = handle_api_config_put(
        State(state(config::Config::default(), true, std::slice::from_ref(&token))),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        "not = valid = toml".to_string(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn handle_api_config_put_rejects_invalid_toml() {
    let response = handle_api_config_put(
        State(state(config::Config::default(), false, &[])),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        "not = valid = toml".to_string(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Invalid TOML"));
}

#[tokio::test]
async fn handle_api_config_put_rejects_type_mismatch_after_toml_parsing() {
    let current = config::Config::default();
    let mut incoming_toml: toml::Value =
        toml::from_str(&toml::to_string_pretty(&current).expect("default config should serialize"))
            .expect("default config should parse as toml value");
    incoming_toml
        .as_table_mut()
        .expect("config root table")
        .insert("default_temperature".to_string(), toml::Value::String("warm".to_string()));
    let body =
        toml::to_string_pretty(&incoming_toml).expect("invalid typed config should serialize");
    let state = state(current, false, &[]);

    let response = handle_api_config_put(
        State(state),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Invalid TOML"));
}

#[tokio::test]
async fn handle_api_config_put_rejects_invalid_config_without_mutating_runtime_state() {
    let current = config::Config::default();
    let expected_host = current.gateway.host.clone();
    let mut incoming = current.clone();
    incoming.gateway.host = " ".to_string();
    let body = toml::to_string_pretty(&incoming).expect("invalid config should serialize");
    let state = state(current, false, &[]);

    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("Invalid config: gateway.host must not be empty")
    );
    assert_eq!(state.config.lock().gateway.host, expected_host);
}

#[tokio::test]
async fn handle_api_config_put_reports_gateway_skey_store_failure_without_mutating_runtime_state() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let invalid_home = temp.path().join("home-file");
    std::fs::write(&invalid_home, "not a directory").expect("invalid home file");
    let _home = EnvGuard::set("HOME", invalid_home.as_os_str());
    let current = config::Config::default();
    let expected_provider = current.default_provider.clone();
    let body = toml::to_string_pretty(&current).expect("default config should serialize");
    let state = state(current, false, &[]);

    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("Failed to save gateway skeys")
    );
    assert_eq!(state.config.lock().default_provider, expected_provider);
}

#[tokio::test]
async fn handle_api_config_put_reports_config_save_failure_without_mutating_runtime_state() {
    let mut current = config::Config::default();
    current.config_path = std::path::PathBuf::new();
    current.default_provider = Some("anthropic".to_string());

    let mut incoming = current.clone();
    incoming.default_provider = Some("openai".to_string());
    let body = toml::to_string_pretty(&incoming).expect("config should serialize as toml");
    let state = state(current, false, &[]);

    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        response_json(response).await["error"].as_str().unwrap().contains("Failed to save config")
    );
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("anthropic"));
}

#[tokio::test]
async fn handle_api_config_put_hydrates_validates_saves_and_updates_runtime_config() {
    let temp = TempDir::new().expect("temp dir");
    let mut current = config::Config::default();
    current.config_path = temp.path().join(".vibewindow.json");
    current.api_key = Some("sk-current".to_string());
    current.gateway.paired_tokens = vec!["paired-current".to_string()];

    let mut incoming = current.clone();
    incoming.default_provider = Some("openai".to_string());
    incoming.api_key = Some(crate::app::agent::gateway::api::secrets::MASKED_SECRET.to_string());
    incoming.gateway.paired_tokens =
        vec![crate::app::agent::gateway::api::secrets::MASKED_SECRET.to_string()];
    let body = toml::to_string_pretty(&incoming).expect("config should serialize as toml");
    let state = state(current, false, &[]);

    let response = handle_api_config_put(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["status"], "ok");
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("openai"));
    assert_eq!(state.config.lock().api_key.as_deref(), Some("sk-current"));
    assert_eq!(state.config.lock().gateway.paired_tokens, vec!["paired-current"]);
    assert!(state.config.lock().config_path.exists());
}

#[tokio::test]
async fn dashboard_config_skeys_are_isolated_by_tenant_scope() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");
    let mut current = config::Config::default();
    current.config_path = temp.path().join("vibewindow.json");
    let state = state(current, false, &[]);

    for (tenant, name, hash_byte) in [(&first, "first-skey", 'a'), (&second, "second-skey", 'b')] {
        let mut incoming = state.config.lock().clone();
        incoming.gateway.skeys = vec![vw_config_types::gateway::GatewaySkey {
            enabled: true,
            skey: None,
            skey_hash: hash_byte.to_string().repeat(64),
            masked_skey: format!("{name}-masked"),
            name: name.to_string(),
            expires_at: None,
        }];
        let body = toml::to_string_pretty(&incoming).expect("serialize config");
        let response = handle_api_config_put(
            State(state.clone()),
            Extension(tenant.clone()),
            HeaderMap::new(),
            body,
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    for (tenant, expected_name) in [(&first, "first-skey"), (&second, "second-skey")] {
        let response = handle_api_config_get(
            State(state.clone()),
            Extension(tenant.clone()),
            HeaderMap::new(),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        let config: config::Config =
            toml::from_str(body["content"].as_str().expect("toml content"))
                .expect("parse config response");
        assert_eq!(config.gateway.skeys[0].name, expected_name);
    }
}

#[tokio::test]
async fn handle_api_config_get_accepts_valid_bearer_token_when_pairing_is_enabled() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let _home = EnvGuard::set("HOME", temp.path().as_os_str());
    let token = "paired-token".to_string();
    let response = handle_api_config_get(
        State(state(config::Config::default(), true, std::slice::from_ref(&token))),
        Extension(TenantScope::default()),
        bearer_headers(&token),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["format"], "toml");
}

#[tokio::test]
async fn config_patch_stateful_rejects_invalid_project_patch_without_mutating_runtime_config() {
    let temp = TempDir::new().expect("temp dir");
    let _config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let _workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    let current = config::Config::default();
    let expected_provider = current.default_provider.clone();
    let state = state(current, false, &[]);

    let error = config_patch_stateful(
        Extension(TenantScope::default()),
        State(state.clone()),
        instance_query(&temp),
        HeaderMap::new(),
        Json(serde_json::json!("invalid-patch")),
    )
    .await
    .expect_err("non-object project patch should be rejected");

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert!(error.message.contains("config patch must be a JSON object"));
    assert_eq!(state.config.lock().default_provider, expected_provider);
}
