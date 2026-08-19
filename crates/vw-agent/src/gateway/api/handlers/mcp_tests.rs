use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use parking_lot::Mutex;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

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

fn state(config: Config, require_pairing: bool, paired_tokens: &[&str]) -> AppState {
    let paired_tokens = paired_tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>();
    AppState {
        config: Arc::new(Mutex::new(config)),
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
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn config_for_workspace(workspace: &std::path::Path) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_path_buf();
    config
}

fn write_workspace_config(tmp: &TempDir, value: serde_json::Value) -> std::path::PathBuf {
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&value).expect("json"),
    )
    .expect("write config");
    workspace
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("body should be json")
}

#[tokio::test]
async fn handle_api_mcp_inspect_returns_disabled_server_metadata() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = write_workspace_config(
        &tmp,
        json!({
            "mcpServers": [{
                "name": "docs",
                "type": "http",
                "url": "http://127.0.0.1:9/mcp",
                "enabled": false,
                "required": true,
                "timeout": 1,
                "allowedTools": ["search"],
                "deniedTools": ["delete"]
            }]
        }),
    );

    let response = handle_api_mcp_inspect(
        State(state(config_for_workspace(&workspace), false, &[])),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let server = &body["servers"][0];
    assert_eq!(server["name"], "docs");
    assert_eq!(server["transport"], "http");
    assert_eq!(server["enabled"], false);
    assert_eq!(server["required"], true);
    assert_eq!(server["connection_status"], "disabled");
    assert_eq!(server["allowed_tools"], json!(["search"]));
    assert_eq!(server["denied_tools"], json!(["delete"]));
    assert!(server.get("tools").is_none());
}

#[tokio::test]
async fn handle_api_mcp_inspect_rejects_missing_bearer_when_pairing_required() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = write_workspace_config(&tmp, json!({ "mcpServers": [] }));

    let response = handle_api_mcp_inspect(
        State(state(config_for_workspace(&workspace), true, &["paired"])),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn handle_api_mcp_inspect_maps_invalid_workspace_config_to_bad_request() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(workspace.join(".vwacprc.json"), "{ invalid json")
        .expect("invalid workspace config");

    let response = handle_api_mcp_inspect(
        State(state(config_for_workspace(&workspace), false, &[])),
        HeaderMap::new(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert!(
        body["error"]
            .as_str()
            .expect("error message")
            .starts_with("Failed to inspect MCP servers:")
    );
}
