use super::super::test_support;
use vw_api_types::id::TenantScope;
#[tokio::test]
async fn cc04_universal_sdk_wire_and_error_redaction() {
    let config = serde_json::json!({"name":"Fixture","providerType":"custom","apps":{},"baseUrl":"https://example.com"});
    let summary = serde_json::json!({"id":"shared","config":config,"configured":true});
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, summary.clone()),
            (200, summary.clone()),
            (200, serde_json::json!([summary])),
            (200, serde_json::json!(true)),
            (403, serde_json::json!({"message":"fixture-secret /private/path"})),
            (409, serde_json::json!({"message":"fixture-secret"})),
        ],
        TenantScope::new("p", "a").unwrap(),
    );
    let req = serde_json::from_value(
        serde_json::json!({"config":config,"apiKey":{"action":"replace","value":"fake-key"}}),
    )
    .unwrap();
    server.client().universal_provider_save("shared", &req).await.unwrap();
    let wire = server.take_request();
    assert_eq!(wire.method, "PUT");
    assert_eq!(wire.path, "/v1/universal-providers/shared");
    assert_eq!(wire.header("x-company-id"), Some("a"));
    assert_eq!(wire.body, serde_json::to_value(req).unwrap());
    server.client().universal_provider_get("shared").await.unwrap();
    assert_eq!(server.take_request().method, "GET");
    server.client().universal_providers_list().await.unwrap();
    assert_eq!(server.take_request().path, "/v1/universal-providers");
    assert!(server.client().universal_provider_delete("shared").await.unwrap());
    assert_eq!(server.take_request().method, "DELETE");
    for code in ["403", "409"] {
        let error = server.client().universal_provider_get("shared").await.unwrap_err();
        assert!(error.contains(code));
        assert!(!error.contains("fixture-secret"));
        assert!(!error.contains("/private/path"));
        server.take_request();
    }
    assert!(server.client().universal_provider_get("../bad").await.is_err());
    server.join();
}
