use vw_api_types::{
    agent_apps::AppId,
    app_providers::{ProviderProtocol, create::*},
};
fn request() -> AppProviderCreate {
    AppProviderCreate {
        request_options: None,
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: ProviderSecretWrite::Replace { value: "fixture-private-key".into() },
        base_url: "https://fixture.example.test".into(),
        model: "fixture-model".into(),
        add_to_live: false,
    }
}
use super::super::test_support;
use vw_api_types::id::TenantScope;
#[tokio::test]
async fn cc04_create_sdk_sends_explicit_intent_and_tenant_to_create_route() {
    let server = test_support::server_with_tenant_scope(
        vec![(
            201,
            serde_json::json!({"id":"new","appId":"claude","name":"fixture","configured":true}),
        )],
        TenantScope::new("p", "a").unwrap(),
    );
    let result =
        server.client().app_provider_create(AppId::Claude, "new", &request()).await.unwrap();
    assert_eq!(result.id, "new");
    let wire = server.take_request();
    assert_eq!(wire.method, "POST");
    assert_eq!(wire.path, "/v1/app-providers/claude/new");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.body["addToLive"], false);
    assert_eq!(wire.body["apiKey"]["action"], "replace");
    server.join();
}

#[tokio::test]
async fn cc04_create_sdk_errors_preserve_status_without_raw_payload() {
    let server = test_support::server_with_tenant_scope(
        vec![
            (409, serde_json::json!({"message":"fixture-private-key"})),
            (403, serde_json::json!({"arbitrary":"fixture-private-key"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    for status in ["409", "403"] {
        let error = server
            .client()
            .app_provider_create(AppId::Claude, "new", &request())
            .await
            .unwrap_err();
        assert!(error.contains(status));
        assert!(!error.contains("fixture-private-key"));
        server.take_request();
    }
    server.join();
}
