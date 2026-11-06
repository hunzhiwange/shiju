use super::super::test_support;
use vw_api_types::{
    agent_apps::AppId, app_providers::icons::AppProviderIconChange as Change, id::TenantScope,
};
#[tokio::test]
async fn cc04_icons_sdk_wire_and_redaction() {
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, serde_json::json!({"data":null})),
            (200, serde_json::json!({"data":null})),
            (403, serde_json::json!({"message":"fixture-secret /private/path"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    assert!(
        server.client().app_provider_icon(AppId::Claude, "source").await.unwrap().data.is_none()
    );
    let wire = server.take_request();
    assert_eq!(wire.method, "GET");
    assert_eq!(wire.path, "/v1/app-providers/claude/source/icon");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    let change = Change::Select { icon_id: "fixture".into() };
    server.client().app_provider_icon_change(AppId::Claude, "source", &change).await.unwrap();
    let wire = server.take_request();
    assert_eq!(wire.method, "PUT");
    assert_eq!(wire.body, serde_json::to_value(&change).unwrap());
    let error = server.client().app_provider_icon(AppId::Claude, "source").await.unwrap_err();
    assert!(error.contains("403"));
    assert!(!error.contains("fixture-secret"));
    assert!(!error.contains("/private/path"));
    server.take_request();
    server.join();
}
