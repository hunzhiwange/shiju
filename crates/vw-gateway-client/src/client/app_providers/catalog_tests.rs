use super::super::test_support;
use vw_api_types::{agent_apps::AppId, id::TenantScope};
#[tokio::test]
async fn cc04_catalog_sdk_lists_without_secret_body() {
    let server = test_support::server_with_tenant_scope(
        vec![(200, serde_json::json!([]))],
        TenantScope::new("p", "a").unwrap(),
    );
    assert!(server.client().app_provider_list(AppId::Claude).await.unwrap().is_empty());
    let request = server.take_request();
    assert_eq!(request.path, "/v1/app-providers/claude");
    assert_eq!(request.header("x-company-id"), Some("a"));
    assert!(request.body.is_null());
    server.join();
}
#[tokio::test]
async fn cc04_catalog_sdk_encodes_host_and_preserves_permission_errors() {
    use vw_api_types::app_providers::catalog::{ProviderCatalog, ProviderSelection};
    let value = serde_json::json!({"appId":"opencode", "rows":[], "direct":{"status":"not_applicable"}, "inUse":{"status":"not_applicable"}, "nativeRead":null});
    let server = test_support::server_with_tenant_scope(
        vec![(200, value), (403, serde_json::json!({"secret":"must-not-surface"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let result: ProviderCatalog = server
        .client()
        .app_provider_catalog(AppId::OpenCode, "host /?&", "fixture-configuration-read-token-32")
        .await
        .unwrap();
    assert_eq!(result.in_use, ProviderSelection::NotApplicable);
    let error = server
        .client()
        .app_provider_catalog(AppId::Kimi, "other", "fixture-configuration-read-token-32")
        .await
        .unwrap_err();
    assert!(matches!(error, crate::AgentAppsReadError::Http { status: 403, .. }));
    assert!(!error.to_string().contains("must-not-surface"));
    let request = server.take_request();
    assert_eq!(
        request.path,
        "/v1/app-providers/opencode/catalog/state?targetHostId=host+%2F%3F%26"
    );
    assert_eq!(request.method, "GET");
    assert_eq!(request.header("x-agent-apps-token"), Some("fixture-configuration-read-token-32"));
    assert_eq!(request.header("x-company-id"), Some("a"));
    assert!(request.body.is_null());
    server.take_request();
    server.join();
}
