use super::*;
use vw_api_types::app_providers::opencode::*;
#[tokio::test]
async fn cc04_opencode_sdk_rejects_path_identity_without_request() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let error = client
        .opencode_provider_add(
            "../escape",
            &OpenCodeTarget { target_host_id: "host".into() },
            "fake-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-token"));
}

#[tokio::test]
async fn cc04_opencode_sdk_sends_scoped_add_and_discards_error_body() {
    use super::super::test_support;
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"fake-response-secret"}))],
        vw_api_types::id::TenantScope::new("p", "a").unwrap(),
    );
    let error = server
        .client()
        .opencode_provider_add(
            "one",
            &OpenCodeTarget { target_host_id: "host".into() },
            "fake-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-response-secret"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/app-providers/opencode/one/add");
    assert_eq!(request.body, serde_json::json!({"targetHostId":"host"}));
    server.join();
}
