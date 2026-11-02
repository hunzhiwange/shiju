//! Explicit real gateway process for native desktop plugin acceptance.
use crate::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::{
    config::Config, memory::NoneMemory, providers::Provider, security::pairing::PairingGuard,
};
use axum::Router;
use parking_lot::Mutex;
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
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
fn app(root: &Path, config_dir: &Path) -> Router {
    let mut config = Config::default();
    config.config_path = config_dir.join("config.json");
    config.workspace_dir = root.join("workspace");
    let state = AppState {
        config: Arc::new(Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "fixture".into(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(true, &["window-fixture-token".into()])),
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

#[tokio::test]
#[ignore = "launched explicitly by native plugin window harness"]
async fn plugin_window_gateway_fixture() {
    let root = std::path::PathBuf::from(
        std::env::var("SHIJU_PLUGIN_WINDOW_ROOT").expect("explicit isolated fixture root"),
    );
    assert!(root.is_absolute() && root.is_dir());
    let source = root.join("source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(root.join("workspace")).unwrap();
    std::fs::create_dir_all(root.join("empty-bin")).unwrap();
    std::fs::write(source.join("plugin.json"),json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"fixture","version":"1","description":"中文插件 · 原包依赖诊断","extensions":{"com.openai":{"interface":{"displayName":"中文插件：运行依赖待确认"},"hooks":{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"printf MUST_NOT_RUN"}]}]}}}}}).to_string()).unwrap();
    std::fs::write(source.join("mcp.json"),json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"示例服务":{"type":"stdio","command":"node","args":["${PLUGIN_ROOT}/server.js"],"env":{"PATH":root.join("empty-bin")}}}}).to_string()).unwrap();
    std::fs::write(source.join("server.js"), "require('fs').writeFileSync('MUST_NOT_RUN','bad')")
        .unwrap();
    let original = std::fs::read(source.join("plugin.json")).unwrap();
    crate::plugins::install_plugin_from_marketplace(
        source.to_str().unwrap(),
        &root.join("config/plugins"),
        "catalog",
    )
    .unwrap();
    // A long Unicode package location exercises actual default-font shaping and path wrapping.
    let long =
        root.join("中文长路径abcdefghijklmnop".repeat(5)).join("中文目录qrstuvwxyz".repeat(5));
    std::fs::create_dir_all(&long).unwrap();
    std::fs::rename(root.join("config/plugins"), long.join("plugins")).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = app(&root, &long);
    std::fs::write(
        root.join("ready.json"),
        json!({"port":port,"source":source,"long_config":long}).to_string(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap();
    });
    let started = std::time::Instant::now();
    while !root.join("stop").exists() {
        assert!(started.elapsed() < Duration::from_secs(180));
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    server.abort();
    let _ = server.await;
    assert_eq!(std::fs::read(source.join("plugin.json")).unwrap(), original);
    assert!(!source.join("MUST_NOT_RUN").exists());
    std::fs::write(
        root.join("backend-result.json"),
        r#"{"original_unchanged":true,"script_not_executed":true}"#,
    )
    .unwrap();
}
