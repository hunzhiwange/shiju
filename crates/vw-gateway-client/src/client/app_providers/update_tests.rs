use super::super::test_support;
use vw_api_types::id::TenantScope;
use vw_api_types::{agent_apps::AppId, app_providers::update::AppProviderUpdate};
#[tokio::test]
async fn cc04_update_sdk_sends_explicit_intent_and_tenant_to_update_route() {
    let server = test_support::server_with_tenant_scope(
        vec![(
            200,
            serde_json::json!({"id":"new","appId":"claude","name":"fixture","configured":true}),
        )],
        TenantScope::new("p", "a").unwrap(),
    );
    let result = server
        .client()
        .app_provider_update(
            AppId::Claude,
            "new",
            &AppProviderUpdate { name: Some("edited".into()), ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!(result.id, "new");
    let wire = server.take_request();
    assert_eq!(wire.method, "PATCH");
    assert_eq!(wire.path, "/v1/app-providers/claude/new");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert!(wire.body.get("model").is_none());
    assert_eq!(wire.body["name"], "edited");
    assert_eq!(wire.body["apiKey"]["action"], "keep");
    server.join();
}

#[tokio::test]
async fn cc04_update_sdk_errors_preserve_status_without_raw_payload() {
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
            .app_provider_update(
                AppId::Claude,
                "new",
                &AppProviderUpdate { name: Some("edited".into()), ..Default::default() },
            )
            .await
            .unwrap_err();
        assert!(error.contains(status));
        assert!(!error.contains("fixture-private-key"));
        server.take_request();
    }
    server.join();
}
