use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::env_restore::*, id::TenantScope};
#[tokio::test]
async fn cc04_env_restore_sdk_encodes_backup_ids_and_redacts_denial() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"hidden"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let request = EnvRestoreRequest { target_host_id: "host".into(), backup_id: "opaque".into() };
    let error = server
        .client()
        .provider_env_restore(AppId::Claude, &request, "fixture-write-token-over-32-chars")
        .await
        .unwrap_err();
    assert!(matches!(error, crate::AgentAppsReadError::Http { status: 403, .. }));
    assert!(!error.to_string().contains("hidden"));
    let captured = server.take_request();
    assert_eq!(captured.path, "/v1/app-providers/claude/environment/restore");
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.body, serde_json::to_value(request).unwrap());
    assert_eq!(captured.header("x-company-id"), Some("a"));
    server.join();
}
