// 覆盖集成目录、凭据更新、认证及修订冲突处理。

use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::broadcast;

struct HomeEnvGuard(Option<OsString>);

impl HomeEnvGuard {
    fn set(home: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", home);
        }
        Self(previous)
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.0 {
                std::env::set_var("HOME", previous);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}

// 修订号在凭据写入前校验，防止并发设置请求覆盖较新的集成配置。
#[test]
fn exported_handlers_are_available() {
    let _ = handle_api_integrations;
    let _ = handle_api_integrations_settings;
    let _ = handle_api_integration_credentials_put;
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn config_in(temp: &TempDir) -> Config {
    let mut config = Config::default();
    config.config_path = temp.path().join("vibewindow.json");
    config.workspace_dir = temp.path().join("workspace");
    config
}

fn state_with(config: Config, require_pairing: bool, token: Option<&str>) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    let paired_tokens = token.map(|token| vec![token.to_string()]).unwrap_or_default();
    AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, &paired_tokens)),
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
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("valid auth header"),
    );
    headers
}

async fn response_json(response: Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    let value = serde_json::from_slice(&bytes).expect("response should be json");
    (status, value)
}

#[tokio::test]
async fn integrations_requires_auth_when_pairing_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), true, Some("paired-token"));

    let (status, value) = response_json(
        handle_api_integrations(State(state), HeaderMap::new()).await.into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value["error"].as_str().unwrap_or_default().contains("Unauthorized"));
}

#[tokio::test]
async fn integration_settings_requires_auth_when_pairing_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), true, Some("paired-token"));

    let (status, value) = response_json(
        handle_api_integrations_settings(State(state), HeaderMap::new()).await.into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value["error"].as_str().unwrap_or_default().contains("Unauthorized"));
}

#[tokio::test]
async fn integrations_lists_registry_entries_with_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut config = config_in(&temp);
    config.default_provider = Some("openai".to_string());
    let state = state_with(config, false, None);

    let (status, value) = response_json(
        handle_api_integrations(State(state), HeaderMap::new()).await.into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let integrations = value["integrations"].as_array().expect("integrations array");
    assert!(integrations.iter().all(|entry| {
        entry["name"].as_str().is_some_and(|value| !value.is_empty())
            && entry["description"].as_str().is_some_and(|value| !value.is_empty())
            && entry["category"].as_str().is_some_and(|value| !value.is_empty())
            && entry["status"].as_str().is_some_and(|value| !value.is_empty())
    }));
    assert!(integrations.iter().any(|entry| {
        entry["name"] == "OpenAI" && entry["status"] == serde_json::json!("Active")
    }));
}

#[tokio::test]
async fn integration_settings_include_revision_and_active_provider() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut config = config_in(&temp);
    config.default_provider = Some("gemini".to_string());
    config.api_key = Some("test-key".to_string());
    let state = state_with(config, false, None);

    let (status, value) = response_json(
        handle_api_integrations_settings(State(state), HeaderMap::new()).await.into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(value["revision"].as_str().is_some_and(|revision| !revision.is_empty()));
    assert_eq!(value["active_default_provider_integration_id"], "google");
    assert!(value["integrations"].as_array().is_some_and(|items| !items.is_empty()));
    assert!(!value.to_string().contains("test-key"));
}

#[tokio::test]
async fn integration_credentials_put_requires_auth_before_processing_body() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), true, Some("paired-token"));
    let body = IntegrationCredentialsUpdateBody {
        revision: Some("stale".to_string()),
        fields: BTreeMap::new(),
    };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("missing".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value["error"].as_str().unwrap_or_default().contains("Unauthorized"));
    assert!(value.get("revision").is_none());
}

// 拒绝路径除状态码外还检查状态保持不变，防止错误响应掩盖已发生的副作用。
#[tokio::test]
async fn integration_credentials_put_rejects_stale_revision() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), false, None);
    let body = IntegrationCredentialsUpdateBody {
        revision: Some("stale".to_string()),
        fields: BTreeMap::new(),
    };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(value["error"], "Integration settings are out of date. Refresh and retry.");
    assert!(value["revision"].as_str().is_some());
}

#[tokio::test]
async fn integration_credentials_put_maps_unknown_integration_to_not_found() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), false, None);
    let body = IntegrationCredentialsUpdateBody { revision: None, fields: BTreeMap::new() };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("missing".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(value["error"], "Unknown integration id: missing");
}

#[tokio::test]
async fn integration_credentials_put_maps_unsupported_api_url_to_internal_server() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), false, None);
    let mut fields = BTreeMap::new();
    fields.insert("api_url".to_string(), "https://example.invalid".to_string());
    let body = IntegrationCredentialsUpdateBody { revision: None, fields };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(value["error"].as_str().unwrap_or_default().contains("does not support api_url"));
}

#[tokio::test]
async fn integration_credentials_put_maps_unknown_field_to_bad_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = state_with(config_in(&temp), false, None);
    let mut fields = BTreeMap::new();
    fields.insert("token".to_string(), "redacted-fixture".to_string());
    let body = IntegrationCredentialsUpdateBody { revision: None, fields };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"], "Unsupported field 'token' for integration 'openai'");
}

#[tokio::test]
async fn integration_credentials_put_maps_invalid_update_to_bad_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut config = config_in(&temp);
    config.gateway.host.clear();
    let state = state_with(config, false, None);
    let body = IntegrationCredentialsUpdateBody { revision: None, fields: BTreeMap::new() };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        value["error"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Invalid integration config update: gateway.host must not be empty")
    );
}

#[tokio::test]
async fn integration_credentials_put_reports_unchanged_when_revision_is_same() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut config = config_in(&temp);
    config.default_provider = Some("openai".to_string());
    let revision = crate::app::agent::gateway::api::integrations::config_revision(&config);
    let state = state_with(config, false, None);
    let body =
        IntegrationCredentialsUpdateBody { revision: Some(revision), fields: BTreeMap::new() };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["unchanged"], true);
}

#[tokio::test]
async fn integration_credentials_put_saves_and_updates_state() {
    let _async_env_lock = crate::test_support::global_async_env_lock().lock().await;
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = tempfile::tempdir().expect("tempdir");
    let _home = HomeEnvGuard::set(temp.path());
    let state = state_with(config_in(&temp), false, None);
    let mut fields = BTreeMap::new();
    fields.insert("api_key".to_string(), " sk-test ".to_string());
    fields.insert("default_model".to_string(), "gpt-5.2".to_string());
    let body = IntegrationCredentialsUpdateBody { revision: None, fields };

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state.clone()),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["status"], "ok");
    assert_eq!(state.config.lock().default_provider.as_deref(), Some("openai"));
    assert_eq!(state.config.lock().api_key.as_deref(), Some("sk-test"));
    let store = crate::app::agent::config::SystemConfigStore::new(
        crate::global_database::GlobalDatabase::for_home(temp.path()).expect("global database"),
    )
    .expect("system config store");
    let payload = store
        .get(&vw_api_types::id::TenantScope::default())
        .expect("stored config lookup")
        .expect("stored config payload");
    let stored: serde_json::Value = serde_json::from_str(&payload).expect("stored config json");
    assert_eq!(stored["default_provider"], "openai");
    assert_eq!(stored["default_model"], "gpt-5.2");
}

#[tokio::test]
async fn integration_credentials_put_accepts_valid_bearer_token() {
    let _async_env_lock = crate::test_support::global_async_env_lock().lock().await;
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = tempfile::tempdir().expect("tempdir");
    let _home = HomeEnvGuard::set(temp.path());
    let state = state_with(config_in(&temp), true, Some("paired-token"));
    let body = IntegrationCredentialsUpdateBody { revision: None, fields: BTreeMap::new() };

    let (status, _value) = response_json(
        handle_api_integration_credentials_put(
            State(state),
            auth_headers("paired-token"),
            AxumPath("openai".to_string()),
            JsonResponse(body),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn integration_credentials_put_reports_config_persistence_failure() {
    let _async_env_lock = crate::test_support::global_async_env_lock().lock().await;
    let _env_lock =
        crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = tempfile::tempdir().expect("tempdir");
    let invalid_home = temp.path().join("not-a-directory");
    std::fs::write(&invalid_home, "fixture").expect("invalid home fixture");
    let _home = HomeEnvGuard::set(&invalid_home);
    let state = state_with(config_in(&temp), false, None);
    let mut fields = BTreeMap::new();
    fields.insert("api_key".to_string(), "new-key".to_string());

    let (status, value) = response_json(
        handle_api_integration_credentials_put(
            State(state.clone()),
            HeaderMap::new(),
            AxumPath("openai".to_string()),
            JsonResponse(IntegrationCredentialsUpdateBody { revision: None, fields }),
        )
        .await
        .into_response(),
    )
    .await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        value["error"]
            .as_str()
            .is_some_and(|message| message.starts_with("Failed to save config:"))
    );
    assert!(state.config.lock().api_key.is_none());
}
