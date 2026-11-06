use super::super::test_support;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::endpoints::{
        AppProviderEndpointChange as Change, AppProviderEndpointKind as Kind,
    },
    id::TenantScope,
};
#[tokio::test]
async fn cc04_endpoints_sdk_wire_and_error_redaction() {
    let rows = serde_json::json!([{"url":"https://fixture.test/v1","kind":"baseUrl","addedAt":10,"lastUsed":null}]);
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, rows.clone()),
            (200, rows),
            (403, serde_json::json!({"message":"fixture-secret"})),
            (409, serde_json::json!({"message":"fixture-secret"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    let change = Change::Add { url: "https://fixture.test/v1".into(), kind: Kind::BaseUrl };
    let result = server
        .client()
        .app_provider_endpoint_change(AppId::Claude, "source", &change)
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    let wire = server.take_request();
    assert_eq!(wire.method, "POST");
    assert_eq!(wire.path, "/v1/app-providers/claude/source/endpoints");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.body, serde_json::to_value(&change).unwrap());
    assert_eq!(
        server.client().app_provider_endpoints(AppId::Claude, "source").await.unwrap(),
        result
    );
    let wire = server.take_request();
    assert_eq!(wire.method, "GET");
    assert_eq!(wire.path, "/v1/app-providers/claude/source/endpoints");
    let err = server
        .client()
        .app_provider_endpoint_change(AppId::Claude, "source", &change)
        .await
        .unwrap_err();
    assert!(err.contains("403"));
    assert!(!err.contains("fixture-secret"));
    server.take_request();
    let err = server.client().app_provider_endpoints(AppId::Claude, "source").await.unwrap_err();
    assert!(err.contains("409"));
    assert!(!err.contains("fixture-secret"));
    server.take_request();
    server.join();
}
