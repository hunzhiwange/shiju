use super::super::test_support;
use vw_api_types::{agent_apps::AppId, app_providers::delete::AppProviderDelete, id::TenantScope};
#[tokio::test]
async fn cc04_delete_sdk_scopes_request_and_redacts_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, serde_json::json!({"deleted":true})),
            (409, serde_json::json!({"secret":"fixture-secret"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    let request = AppProviderDelete { target_host_id: "host".into() };
    assert!(
        server
            .client()
            .app_provider_delete(AppId::OpenCode, "unused", &request, "fixture-token")
            .await
            .unwrap()
            .deleted
    );
    let wire = server.take_request();
    assert_eq!(wire.method, "DELETE");
    assert_eq!(wire.path, "/v1/app-providers/opencode/unused");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.header("x-agent-apps-token"), Some("fixture-token"));
    assert_eq!(wire.body, serde_json::json!({"targetHostId":"host"}));
    let error = server
        .client()
        .app_provider_delete(AppId::OpenCode, "used", &request, "fixture-token")
        .await
        .unwrap_err();
    assert!(error.contains("409"));
    assert!(!error.contains("fixture-secret"));
    server.take_request();
    server.join();
}
