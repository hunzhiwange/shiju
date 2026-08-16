use super::*;
use crate::app::agent::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::hooks::build_hook_runtime;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::ToolResult;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tower::util::ServiceExt;
use vw_config_types::plugins::PluginsConfig;

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

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(value).expect("json bytes")).expect("write json");
}

fn test_config(root: &Path, hook_args: Vec<String>) -> crate::config::Config {
    let workspace_dir = root.join("workspace");
    let config_dir = root.join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/demo-plugin");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    write_json(
        &plugin_root.join(".vibewindow-plugin/plugin.json"),
        &json!({
            "name": "demo-plugin",
            "version": "0.1.0",
            "description": "hook API test plugin",
            "hooks": "./hooks/hooks.json"
        }),
    );
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        &json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "/bin/sh",
                "args": hook_args,
                "working_dir": "."
            }]
        }),
    );

    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace_dir;
    config.config_path = config_dir.join("vibewindow.json");
    config.hooks.enabled = true;
    config.hooks.builtin.command_logger = false;
    config.hooks.plugins.enabled = true;
    config.plugins = PluginsConfig::default();
    config
}

fn state(config: crate::config::Config) -> AppState {
    let hook_runtime = build_hook_runtime(&config).expect("hook runtime");
    AppState {
        config: Arc::new(Mutex::new(config)),
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: Some(hook_runtime),
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    serde_json::from_slice(&bytes).expect("response json")
}

async fn hook_catalog(app: &axum::Router) -> Vec<vw_gateway_client::HookCatalogEntryDto> {
    let response =
        app.clone().oneshot(Request::get("/hooks").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_value(response_json(response).await).expect("hook catalog")
}

async fn mutate(app: &axum::Router, path: &str, plugin_id: &str, hook_hash: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "plugin_id": plugin_id,
                        "hook_hash": hook_hash
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn hook_trust_rejects_stale_or_unknown_hash() {
    let root = tempfile::tempdir().expect("temp dir");
    let config = test_config(root.path(), vec!["-c".into(), "exit 0".into()]);
    let hooks_path = config.workspace_dir.join(".vibewindow/plugins/demo-plugin/hooks/hooks.json");
    let app = router().with_state(state(config));
    let original = hook_catalog(&app).await.remove(0);

    write_json(
        &hooks_path,
        &json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "/bin/sh",
                "args": ["-c", "printf changed >/dev/null"],
                "working_dir": "."
            }]
        }),
    );

    assert_eq!(
        mutate(&app, "/hooks/trust", "demo-plugin", &original.hook_hash).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        mutate(&app, "/hooks/trust", "unknown-plugin", &original.hook_hash).await,
        StatusCode::BAD_REQUEST
    );
}

#[cfg(unix)]
#[tokio::test]
async fn hook_trust_mutation_reloads_runtime() {
    let root = tempfile::tempdir().expect("temp dir");
    let marker = root.path().join("hook-ran");
    let config = test_config(root.path(), vec!["-c".into(), format!("touch {}", marker.display())]);
    let state = state(config);
    let runtime = state.hook_runtime.clone().expect("runtime");
    let app = router().with_state(state);
    let hook = hook_catalog(&app).await.remove(0);

    let before = runtime.runner_snapshot().expect("runner");
    let _ = before.run_before_tool_call("shell".into(), json!({})).await;
    assert!(!marker.exists(), "untrusted hook must not be registered");

    assert_eq!(
        mutate(&app, "/hooks/trust", &hook.plugin_id, &hook.hook_hash).await,
        StatusCode::OK
    );
    let trusted = runtime.runner_snapshot().expect("trusted runner");
    let _ = trusted.run_before_tool_call("shell".into(), json!({})).await;
    assert!(marker.exists(), "trust must publish a runtime containing the hook");

    fs::remove_file(&marker).expect("remove marker");
    assert_eq!(
        mutate(&app, "/hooks/revoke", &hook.plugin_id, &hook.hook_hash).await,
        StatusCode::OK
    );
    let revoked = runtime.runner_snapshot().expect("revoked runner");
    let _ = revoked.run_before_tool_call("shell".into(), json!({})).await;
    assert!(!marker.exists(), "revoke must publish a runtime without the hook");
}

#[tokio::test]
async fn hook_audit_response_omits_sensitive_payloads() {
    let root = tempfile::tempdir().expect("temp dir");
    let mut config = test_config(root.path(), vec!["-c".into(), "exit 0".into()]);
    config.hooks.builtin.command_logger = true;
    let state = state(config);
    let runtime = state.hook_runtime.clone().expect("runtime");
    runtime
        .runner_snapshot()
        .expect("runner")
        .fire_after_tool_call(
            "shell",
            &ToolResult {
                success: false,
                output: "secret-result".into(),
                error: Some("secret-error".into()),
            },
            Duration::from_millis(12),
        )
        .await;
    let response = router()
        .with_state(state)
        .oneshot(Request::get("/hooks/audit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let encoded = serde_json::to_string(&body).unwrap();

    assert_eq!(body[0]["tool_name"], "shell");
    assert_eq!(body[0]["duration_ms"], 12);
    assert_eq!(body[0]["success"], false);
    assert!(!encoded.contains("secret-result"));
    assert!(!encoded.contains("secret-error"));
    assert!(body[0].get("args").is_none());
    assert!(body[0].get("output").is_none());
    assert!(body[0].get("error").is_none());
}
