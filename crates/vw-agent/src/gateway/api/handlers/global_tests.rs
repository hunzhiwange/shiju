// 覆盖全局配置、健康状态、事件流及租户隔离接口。

use super::*;
use agent_gateway::gateway_test_fixture::{state, use_temp_config_dir};
use axum::body::Body;
use axum::http::Request;
use std::sync::Arc;
use tempfile::TempDir;
use vw_api_transport::id::TenantScope;

const BOOTSTRAP_SOURCE: &str = include_str!("../../bootstrap.rs");

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
    let state: crate::gateway::AppState = state(initial, true, std::slice::from_ref(&token));
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
    agent_gateway::global_config_contract::concurrent_fields_preserve_both_values(
        url, token, tenant, other, &state,
    ).await;
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}
