use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::config::SystemConfigStore;
use crate::app::agent::config::schema::ConfigExt;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use axum::body::to_bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|err| err.into_inner())
}

struct HomeEnvGuard(Option<std::ffi::OsString>);

impl HomeEnvGuard {
    fn set(home: &Path) -> Self {
        let original = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", home) };
        Self(original)
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(original) = &self.0 {
                std::env::set_var("HOME", original);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}

fn stored_config(home: &Path) -> Config {
    let payload = SystemConfigStore::new(
        crate::global_database::GlobalDatabase::for_home(home).expect("database"),
    )
    .expect("config store")
    .get(&vw_api_types::id::TenantScope::default())
    .expect("read config")
    .expect("stored config");
    serde_json::from_str(&payload).expect("parse config")
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
        Ok("ok".into())
    }
}

fn state_with(pairing: PairingGuard, rate_limit: u32, config: Config) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(pairing),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(rate_limit, 100, 1_000)),
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

fn test_config() -> Config {
    let temp = tempfile::tempdir().unwrap().keep();
    let mut config = Config::default();
    config.config_path = temp.join("vibewindow.json");
    config.workspace_dir = temp.join("workspace");
    config
}

fn connect_info(ip: IpAddr) -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::new(ip, 8080))
}

async fn call_pair(state: AppState, code: Option<&str>) -> (StatusCode, serde_json::Value) {
    let mut headers = HeaderMap::new();
    if let Some(code) = code {
        headers.insert("X-Pairing-Code", HeaderValue::from_str(code).unwrap());
    }

    let response =
        handle_pair(State(state), connect_info(IpAddr::V4(Ipv4Addr::LOCALHOST)), headers)
            .await
            .into_response();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

async fn call_pair_code(state: AppState, ip: IpAddr) -> (StatusCode, serde_json::Value) {
    let response = handle_pair_code(State(state), connect_info(ip)).await.into_response();

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test(flavor = "current_thread")]
async fn handle_pair_returns_token_and_persists_when_code_is_valid() {
    let _env_guard = env_lock();
    let home = tempfile::tempdir().unwrap();
    let _home_guard = HomeEnvGuard::set(home.path());
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let config = test_config();
    config.save().await.unwrap();
    let state = state_with(guard, 100, config);

    let (status, body) = call_pair(state.clone(), Some(&code)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["paired"], true);
    assert_eq!(body["persisted"], true);
    let token = body["token"].as_str().expect("token");
    assert!(state.pairing.is_authenticated(token));

    let parsed = stored_config(home.path());
    assert_eq!(parsed.gateway.paired_tokens.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn handle_pair_reports_unpersisted_success_when_config_save_fails() {
    let _env_guard = env_lock();
    let temp = tempfile::tempdir().unwrap();
    let invalid_home = temp.path().join("not-a-directory");
    std::fs::write(&invalid_home, "fixture").unwrap();
    let _home_guard = HomeEnvGuard::set(&invalid_home);
    let mut config = Config::default();
    config.config_path = temp.path().to_path_buf();
    config.workspace_dir = temp.path().join("workspace");

    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let state = state_with(guard, 100, config);

    let (status, body) = call_pair(state, Some(&code)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["paired"], true);
    assert_eq!(body["persisted"], false);
    assert!(body["message"].as_str().unwrap().contains("failed to persist"));
}

#[tokio::test]
async fn handle_pair_rejects_missing_wrong_empty_disabled_and_rate_limited_codes() {
    let state = state_with(PairingGuard::new(true, &[]), 100, test_config());
    assert_eq!(call_pair(state.clone(), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call_pair(state.clone(), Some("wrong")).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call_pair(state, Some("")).await.0, StatusCode::FORBIDDEN);

    let disabled = state_with(PairingGuard::new(false, &[]), 100, test_config());
    assert_eq!(call_pair(disabled, Some("anything")).await.0, StatusCode::FORBIDDEN);

    let limited = state_with(PairingGuard::new(true, &[]), 1, test_config());
    let _ = call_pair(limited.clone(), Some("first")).await;
    let (status, body) = call_pair(limited, Some("second")).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["retry_after"], RATE_LIMIT_WINDOW_SECS);
}

#[tokio::test]
async fn handle_pair_returns_lockout_after_repeated_failures() {
    let state = state_with(PairingGuard::new(true, &[]), 1_000, test_config());
    let mut last = (StatusCode::OK, serde_json::Value::Null);

    for _ in 0..6 {
        last = call_pair(state.clone(), Some("wrong-code")).await;
    }

    assert_eq!(last.0, StatusCode::TOO_MANY_REQUESTS);
    assert!(last.1["retry_after"].as_u64().is_some());
    assert!(last.1["error"].as_str().unwrap().contains("Too many failed attempts"));
}

#[tokio::test]
async fn handle_pair_code_allows_loopback_and_rejects_remote_clients() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let state = state_with(guard, 100, test_config());

    let (status, body) = call_pair_code(state.clone(), IpAddr::V4(Ipv4Addr::LOCALHOST)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["require_pairing"], true);
    assert_eq!(body["paired"], false);
    assert_eq!(body["pairing_code"], code);

    let (status, body) = call_pair_code(state, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body["error"].as_str().unwrap().contains("loopback"));
}

#[tokio::test]
async fn handle_pair_code_omits_code_when_pairing_not_required() {
    let state = state_with(PairingGuard::new(false, &[]), 100, test_config());

    let (status, body) = call_pair_code(state, IpAddr::V4(Ipv4Addr::LOCALHOST)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["require_pairing"], false);
    assert_eq!(body["paired"], false);
    assert!(body["pairing_code"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn persist_pairing_tokens_updates_in_memory_config_and_returns_errors() {
    let _env_guard = env_lock();
    let home = tempfile::tempdir().unwrap();
    let home_guard = HomeEnvGuard::set(home.path());
    let config = test_config();
    config.save().await.unwrap();
    let shared = Arc::new(Mutex::new(config));
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let token = guard.try_pair(&code, "client").await.unwrap().unwrap();

    persist_pairing_tokens(shared.clone(), &guard).await.unwrap();

    let tokens = shared.lock().gateway.paired_tokens.clone();
    assert_eq!(tokens.len(), 1);
    assert_ne!(tokens[0], token);
    assert_eq!(tokens[0].len(), 64);
    assert!(tokens[0].chars().all(|value| value.is_ascii_hexdigit()));
    assert!(guard.is_authenticated(&token));

    let temp = tempfile::tempdir().unwrap();
    drop(home_guard);
    let invalid_home = temp.path().join("not-a-directory");
    std::fs::write(&invalid_home, "fixture").unwrap();
    let _home_guard = HomeEnvGuard::set(&invalid_home);
    let mut bad_config = Config::default();
    bad_config.config_path = temp.path().to_path_buf();
    bad_config.workspace_dir = temp.path().join("workspace");
    let error = persist_pairing_tokens(Arc::new(Mutex::new(bad_config)), &guard).await.unwrap_err();
    assert!(error.to_string().contains("Failed to persist paired tokens"));
}
