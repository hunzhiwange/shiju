// 覆盖会话创建、筛选、状态、待办及实例作用域操作。

use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::{project, session};
use std::path::Path as FsPath;
use axum::{extract::Query, http::HeaderMap};
use tempfile::TempDir;
use gateway_session::host_lifecycle_acceptance::{ensure_test_home, SESSION_OPS_TEST_LOCK};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use vw_api_session::session::{
    GatewaySessionCreateBody,
};

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

fn state() -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
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

fn query_for(directory: &FsPath) -> InstanceQuery {
    InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) }
}

async fn create_session_in(directory: &FsPath, title: &str) -> agent_session::session::Info {
    let body = UiSessionCreateBody {
        session: GatewaySessionCreateBody { parent_id: None, title: Some(title.to_string()) },
        permission: None,
    };
    ui_session_create(Query(query_for(directory)), HeaderMap::new(), Json(Some(body)))
        .await
        .expect("session should be created")
        .0
}

// 同名会话在不同目录中创建，验证筛选、修改和删除始终受实例作用域约束。
#[test]
fn session_handlers_are_available() {
    let _ = ui_session_list;
    let _ = ui_session_status;
    let _ = ui_session_get;
    let _ = ui_session_create;
    let _ = ui_session_patch;
    let _ = ui_session_delete;
    let _ = session_children;
    let _ = session_todo_get;
    let _ = session_todo_put;
    let _ = session_fork;
    let _ = session_reset;
    let _ = session_diff;
    let _ = session_summarize;
    let _ = session_title_generate;
}

#[tokio::test]
async fn gateway_first_response_create_without_title_reserves_an_unpublished_session() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    gateway_session::host_lifecycle_acceptance::gateway_first_response_create_without_title_reserves_an_unpublished_session::<Host>(state()).await;
}

#[tokio::test]
async fn ui_session_list_respects_updated_boundary_and_zero_limit() {
    gateway_session::host_lifecycle_acceptance::ui_session_list_respects_updated_boundary_and_zero_limit::<Host>(state()).await;
}

#[tokio::test]
async fn ui_session_status_preserves_existing_non_idle_state() {
    gateway_session::host_lifecycle_acceptance::ui_session_status_preserves_existing_non_idle_state::<Host>(state()).await;
}

#[tokio::test]
async fn ui_session_patch_ignores_empty_fields_and_uses_header_scope() {
    gateway_session::host_lifecycle_acceptance::ui_session_patch_ignores_empty_fields_and_uses_header_scope::<Host>(state()).await;
}

#[tokio::test]
async fn session_todo_put_replaces_and_clears_saved_items() {
    gateway_session::host_lifecycle_acceptance::session_todo_put_replaces_and_clears_saved_items::<Host>(state()).await;
}

#[tokio::test]
async fn scoped_mutations_do_not_modify_sessions_from_another_instance() {
    gateway_session::host_lifecycle_acceptance::scoped_mutations_do_not_modify_sessions_from_another_instance::<Host>(state()).await;
}

#[tokio::test]
async fn session_ops_cover_lifecycle_filters_and_instance_boundaries() {
    gateway_session::host_lifecycle_acceptance::session_ops_cover_lifecycle_filters_and_instance_boundaries::<Host>(state()).await;
}

#[tokio::test]
async fn session_ops_cover_diff_summary_reset_and_title_errors() {
    gateway_session::host_lifecycle_acceptance::session_ops_cover_diff_summary_reset_and_title_errors::<Host>(state()).await;
}

#[path = "turn_fork_tests.rs"]
mod turn_fork_tests;
