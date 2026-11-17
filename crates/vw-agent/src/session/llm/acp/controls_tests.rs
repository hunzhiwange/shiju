//! Real ACP JSON-RPC child process; record codec round trips use temporary storage.
use super::*;
use llm_runtime::acp::cache::ACP_CLIENT_CACHE;
use serde_json::json;

#[tokio::test]
async fn acp_session_options_follow_capabilities_isolate_tenant_connections() {
    let temp = tempfile::tempdir().unwrap();
    let config = crate::config::schema::AcpAgentConfig {
        command: "unused-fixture-command".into(),
        args: vec![],
        env: Default::default(),
    };
    let options = super::super::ParsedAcpOptions::default();
    let first = crate::session::ui_store::with_tenant_scope(
        vw_api_transport::TenantScope::new("platform", "company-a").unwrap(),
        async {
            super::super::session::get_cached_acp_client("fixture", &config, temp.path(), &options)
        },
    )
    .await;
    let second = crate::session::ui_store::with_tenant_scope(
        vw_api_transport::TenantScope::new("platform", "company-b").unwrap(),
        async {
            super::super::session::get_cached_acp_client("fixture", &config, temp.path(), &options)
        },
    )
    .await;
    assert!(!Arc::ptr_eq(&first.client, &second.client));
    assert_ne!(first.client.tenant, second.client.tenant);
    let mut cache = ACP_CLIENT_CACHE.lock();
    cache.remove(&first.key);
    cache.remove(&second.key);
}

#[tokio::test]
async fn acp_model_source_session_controls_busy_guard_is_session_scoped() {
    let session = format!("controls-{}", uuid::Uuid::new_v4());
    let guard = super::super::lifecycle::try_lock(&session, "/workspace").unwrap();
    assert!(
        super::super::lifecycle::try_lock(&session, "/workspace").unwrap_err().contains("busy")
    );
    assert!(super::super::lifecycle::try_lock(&session, "/other").is_ok());
    let other = crate::session::ui_store::with_tenant_scope(
        vw_api_transport::TenantScope::new("platform", "other-company").unwrap(),
        async { super::super::lifecycle::try_lock(&session, "/workspace") },
    )
    .await;
    assert!(other.is_ok());
    drop(guard);
    assert!(super::super::lifecycle::try_lock(&session, "/workspace").is_ok());
}
