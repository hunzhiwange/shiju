//! HTTP regression coverage using production authentication, tenant and connection middleware.
use model_types::Provider;
use crate::app::agent::gateway::{self, AppState, GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::{
    config::Config, memory::NoneMemory, security::pairing::PairingGuard,
};
use parking_lot::Mutex;
use std::{net::SocketAddr, sync::Arc, time::Duration};
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

// 状态 fixture 关闭外部依赖，只保留路由装配所需的最小共享组件。
fn state() -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(true, &["drag-test-key".into()])),
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

use agent_gateway::file_drag_contract::{self as contract, Server};
async fn server(bootstrap: bool) -> Server {
    server_on(bootstrap, "127.0.0.1").await
}
async fn server_on(bootstrap: bool, host: &str) -> Server {
    if bootstrap {
        let state = state();
        let router = gateway::bootstrap::client_router(
            state.clone(),
            axum::Router::new(),
            axum::Router::new(),
        )
        .with_state(state);
        let listener = tokio::net::TcpListener::bind((host, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .map_err(|error| gateway::ApiError::internal(error.to_string()))
        });
        Server { addr, handle }
    } else {
        let (addr, task) =
            gateway::start(gateway::ServeOptions { hostname: host.into(), port: 0, cors: vec![] })
                .await
                .unwrap();
        Server { addr, handle: task }
    }
}

#[tokio::test]
async fn file_drag_integration_both_listeners_preserve_data_and_reject_conflicts() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        contract::file_drag_integration_both_listeners_preserve_data_and_reject_conflicts(vec![server]).await;
    }
}

#[tokio::test]
async fn file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects() {
    let servers = vec![server(true).await];
    contract::file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects(servers).await;
}

#[cfg(unix)]
#[tokio::test]
async fn file_drag_integration_recursive_link_failure_keeps_source_and_other_items() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        contract::file_drag_integration_recursive_link_failure_keeps_source_and_other_items(vec![server]).await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn file_drag_integration_non_loopback_cannot_spoof_forwarded_headers() {
    for bootstrap in [true, false] {
        let server = server_on(bootstrap, "0.0.0.0").await;
        contract::file_drag_integration_non_loopback_cannot_spoof_forwarded_headers(vec![server]).await;
    }
}

#[tokio::test]
async fn file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        contract::file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant(vec![(bootstrap, server)]).await;
    }
}
