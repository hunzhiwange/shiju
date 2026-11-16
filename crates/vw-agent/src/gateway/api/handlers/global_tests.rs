// 覆盖全局配置、健康状态、事件流及租户隔离接口。

use super::*;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use parking_lot::Mutex;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use vw_api_transport::id::TenantScope;

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
        pairing: Arc::new(PairingGuard::new(require_pairing, tokens)),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

fn use_temp_config_dir(temp: &TempDir) -> (EnvGuard, EnvGuard, EnvGuard) {
    let home = EnvGuard::set("HOME", temp.path().as_os_str());
    let config_dir = EnvGuard::set("VIBEWINDOW_CONFIG_DIR", temp.path().as_os_str());
    let workspace = EnvGuard::unset("VIBEWINDOW_WORKSPACE");
    (home, config_dir, workspace)
}

#[test]
fn gateway_bootstrap_loads_default_tenant_skeys_from_shared_store_without_legacy_fallback() {
    assert!(BOOTSTRAP_SOURCE.contains("load_gateway_skeys_for_tenant"));
    assert!(BOOTSTRAP_SOURCE.contains("TenantScope::default()"));
    assert!(!BOOTSTRAP_SOURCE.contains("load_existing_gateway_skeys"));
    assert!(!BOOTSTRAP_SOURCE.contains("falling back to config"));
}

#[tokio::test]
async fn global_config_patch_stateful_concurrent_fields_preserve_both_values() {
    use crate::config::GLOBAL_CONFIG_PATCH_READ_BARRIER;
    use axum::middleware::{Next, from_fn};
    use serde_json::json;

    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = TempDir::new().expect("temp dir");
    let (_home, _config_dir, _workspace) = use_temp_config_dir(&temp);
    let tenant = TenantScope::new("patch-test", "primary").unwrap();
    let other = TenantScope::new("patch-test", "other").unwrap();
    let token = uuid::Uuid::new_v4().to_string();
    config::update_global_for_tenant(
        &tenant,
        json!({
            "gateway": {"auth_enabled": true, "skeys": [{"skey": token, "enabled": true}]},
            "app_ui": {"system_settings": {"editor_font_size": 14, "terminal_font_size": 14}}
        }),
    )
    .await
    .expect("initialize authenticated tenant");
    let initial = config::get_global_for_tenant(&tenant).await.unwrap();
    let state = state(initial, true, std::slice::from_ref(&token));
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let app = crate::gateway::bootstrap::client_router(state.clone(), Router::new(), Router::new())
        .layer(from_fn(move |request: Request<Body>, next: Next| {
            let barrier = barrier.clone();
            async move {
                if request.headers().contains_key("x-test-concurrent-patch") {
                    GLOBAL_CONFIG_PATCH_READ_BARRIER.scope(barrier, next.run(request)).await
                } else {
                    next.run(request).await
                }
            }
        }))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1/global/config", listener.local_addr().unwrap());
    let server = tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let request = |method, scope: &TenantScope| {
        client
            .request(method, &url)
            .bearer_auth(&token)
            .header(vw_api_transport::PLATFORM_ID_HEADER, scope.platform_id().as_ref())
            .header(vw_api_transport::COMPANY_ID_HEADER, scope.company_id().as_ref())
    };
    let read_fonts = |response: reqwest::Response| async move {
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.json::<Value>().await.expect("JSON response");
        let settings = &body["app_ui"]["system_settings"];
        (settings["editor_font_size"].as_f64(), settings["terminal_font_size"].as_f64())
    };
    let get = |scope| request(reqwest::Method::GET, scope);
    let patch = |body| request(reqwest::Method::PATCH, &tenant).json(&body);
    assert_eq!(client.get(&url).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    let other_before = read_fonts(get(&other).send().await.unwrap()).await;

    for round in 0..20 {
        if round > 0 {
            let reset = patch(json!({
                "app_ui": {"system_settings": {"editor_font_size": 14, "terminal_font_size": 14}}
            }))
            .send()
            .await
            .unwrap();
            assert_eq!(reset.status(), StatusCode::OK);
        }
        let (editor, terminal) = tokio::join!(
            patch(json!({"app_ui": {"system_settings": {"editor_font_size": 24}}}))
                .header("x-test-concurrent-patch", "1")
                .send(),
            patch(json!({"app_ui": {"system_settings": {"terminal_font_size": 19}}}))
                .header("x-test-concurrent-patch", "1")
                .send(),
        );
        assert_eq!(editor.unwrap().status(), StatusCode::OK);
        assert_eq!(terminal.unwrap().status(), StatusCode::OK);
        assert_eq!(
            read_fonts(get(&tenant).send().await.unwrap()).await,
            (Some(24.0), Some(19.0)),
            "persisted fonts in round {round}"
        );
        let cached = state.config.lock();
        let settings = &cached.app_ui.system_settings;
        assert_eq!((settings.editor_font_size, settings.terminal_font_size), (24.0, 19.0));
    }

    let invalid = patch(json!({
        "gateway": {"host": "", "skeys": []},
        "app_ui": {"system_settings": {"editor_font_size": 30}}
    }))
    .send()
    .await
    .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert!(state.pairing.is_authenticated(&token));
    assert_eq!(read_fonts(get(&tenant).send().await.unwrap()).await, (Some(24.0), Some(19.0)));
    assert_eq!(read_fonts(get(&other).send().await.unwrap()).await, other_before);
    {
        let cached = state.config.lock();
        let settings = &cached.app_ui.system_settings;
        assert_eq!((settings.editor_font_size, settings.terminal_font_size), (24.0, 19.0));
    }
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}
