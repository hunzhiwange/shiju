use super::super::test_support;
use vw_api_types::{
    agent_apps::AppId, app_providers::duplicate::AppProviderDuplicate, id::TenantScope,
};
#[tokio::test]
async fn cc04_duplicate_sdk_sends_scoped_copy_and_redacts_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                201,
                serde_json::json!({"id":"copy","appId":"claude","name":"fixture copy","configured":true}),
            ),
            (409, serde_json::json!({"message":"fixture-private-key"})),
            (403, serde_json::json!({"message":"fixture-private-key"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    let input = AppProviderDuplicate { new_id: "copy".into(), name: "fixture copy".into() };
    assert_eq!(
        server.client().app_provider_duplicate(AppId::Claude, "source", &input).await.unwrap().id,
        "copy"
    );
    let wire = server.take_request();
    assert_eq!(wire.method, "POST");
    assert_eq!(wire.path, "/v1/app-providers/claude/source/duplicate");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.body, serde_json::json!({"newId":"copy","name":"fixture copy"}));
    for status in ["409", "403"] {
        let error = server
            .client()
            .app_provider_duplicate(AppId::Claude, "source", &input)
            .await
            .unwrap_err();
        assert!(error.contains(status));
        assert!(!error.contains("fixture-private-key"));
        server.take_request();
    }
    server.join();
}
