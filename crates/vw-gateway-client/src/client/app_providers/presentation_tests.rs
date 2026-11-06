use super::super::test_support;
use vw_api_types::{
    agent_apps::AppId, app_providers::presentation::AppProviderPresentation, id::TenantScope,
};
#[tokio::test]
async fn cc04_presentation_sdk_wire_and_error_redaction() {
    let value = serde_json::json!({"sortIndex":2,"category":"group","notes":"note"});
    let rows = serde_json::json!([{"id":"source","presentation":value}]);
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, value.clone()),
            (200, rows),
            (403, serde_json::json!({"message":"fixture-secret"})),
            (409, serde_json::json!({"message":"fixture-secret"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    let input: AppProviderPresentation = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        server
            .client()
            .app_provider_presentation_set(AppId::Claude, "source", &input)
            .await
            .unwrap(),
        input
    );
    let wire = server.take_request();
    assert_eq!(wire.method, "PUT");
    assert_eq!(wire.path, "/v1/app-providers/claude/source/presentation");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.body, value);
    assert_eq!(
        server.client().app_provider_presentation_list(AppId::Claude).await.unwrap()[0]
            .presentation,
        input
    );
    let wire = server.take_request();
    assert_eq!(wire.method, "GET");
    assert_eq!(wire.path, "/v1/app-providers/claude/presentation/entries");
    let err = server
        .client()
        .app_provider_presentation_set(AppId::Claude, "source", &input)
        .await
        .unwrap_err();
    assert!(err.contains("403"));
    assert!(!err.contains("fixture-secret"));
    server.take_request();
    let err = server.client().app_provider_presentation_list(AppId::Claude).await.unwrap_err();
    assert!(err.contains("409"));
    assert!(!err.contains("fixture-secret"));
    server.take_request();
    server.join();
}
