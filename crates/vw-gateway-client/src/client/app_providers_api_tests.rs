use super::test_support;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
    id::TenantScope,
};
#[tokio::test]
async fn cc_apps_provider_launch_isolation_sdk_uses_domain_identity_and_tenant() {
    let summary =
        serde_json::json!({"id":"selected","appId":"claude","name":"fixture","configured":true});
    let server = test_support::server_with_tenant_scope(
        vec![(200, summary.clone()), (200, summary)],
        TenantScope::new("p", "a").unwrap(),
    );
    let profile = AppProviderWrite {
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: "fixture-key-only".into(),
        base_url: "https://provider.example.test".into(),
        model: "fixture".into(),
    };
    server.client().app_provider_save(AppId::Claude, "selected", &profile).await.unwrap();
    server.client().app_provider_get(AppId::Claude, "selected").await.unwrap();
    for method in ["PUT", "GET"] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, "/v1/app-providers/claude/selected");
        assert_eq!(request.header("x-platform-id"), Some("p"));
        assert_eq!(request.header("x-company-id"), Some("a"));
        if method == "PUT" {
            assert_eq!(request.body["apiKey"], "fixture-key-only");
        } else {
            assert!(request.body.is_null());
        }
    }
    assert!(server.client().app_provider_get(AppId::Claude, "../auth").await.is_err());
    server.join();
}
