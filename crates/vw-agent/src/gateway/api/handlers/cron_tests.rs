// 覆盖定时任务 API 的调度解析、认证及任务生命周期操作。

use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

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

fn state_with_config(config: Arc<parking_lot::Mutex<Config>>, require_pairing: bool) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config,
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(require_pairing, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

fn temp_config(workspace: &std::path::Path) -> Arc<parking_lot::Mutex<Config>> {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_path_buf();
    Arc::new(parking_lot::Mutex::new(config))
}

use gateway_cron::host_contracts as contract;

#[tokio::test]
async fn cron_handlers_reject_requests_when_pairing_is_required() {
    let dir = tempfile::tempdir().unwrap();
    contract::cron_handlers_reject_requests_when_pairing_is_required(cron_state(state_with_config(temp_config(dir.path()), true))).await;
}

#[tokio::test]
async fn cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required() {
    let dir = tempfile::tempdir().unwrap();
    contract::cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required(cron_state(state_with_config(temp_config(dir.path()), true))).await;
}

fn router(state: AppState) -> axum::Router {
    crate::gateway::bootstrap::client_router(state.clone(), axum::Router::new(), axum::Router::new()).with_state(state)
}

#[tokio::test]
async fn cron_history_endpoint_requires_auth() {
    let dir = tempfile::tempdir().unwrap();
    contract::cron_history_endpoint_requires_auth(router(state_with_config(temp_config(dir.path()), true))).await;
}

#[tokio::test]
async fn cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target() {
    let tenant = vw_api_transport::TenantScope::new("cron-api-test", uuid::Uuid::new_v4().to_string()).unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let dir = tempfile::tempdir().unwrap();
        let config = temp_config(dir.path());
        {
            let mut c = config.lock();
            c.config_path = dir.path().join("config.json");
            contract::configure_wecom(&mut c);
        }
        let saved = config.lock().clone();
        crate::config::schema::save_config_for_tenant(&saved, &tenant).await.unwrap();
        contract::cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target(cron_state(state_with_config(config, false))).await;
    }).await;
}

#[tokio::test]
async fn wb_schedule_api_preview_routes_enforce_auth_and_return_candidates() {
    for pairing in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let config = temp_config(dir.path());
        let state = state_with_config(config.clone(), pairing);
        contract::wb_schedule_api_preview_routes_enforce_auth_and_return_candidates(router(state), config, pairing).await;
    }
}
