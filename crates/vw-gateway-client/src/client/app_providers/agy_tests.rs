use vw_api_types::app_providers::agy::*;
#[tokio::test]
async fn cc04_agy_sdk_scopes_card_and_discards_error_payload() {
    let server = super::super::test_support::server_with_tenant_scope(
        vec![(422, serde_json::json!({"secret":"fake-upstream-secret"}))],
        vw_api_types::id::TenantScope::new("p", "a").unwrap(),
    );
    let e = server
        .client()
        .agy_provider_switch("one", &AgyTarget { target_host_id: "host".into() }, "fake-token")
        .await
        .unwrap_err();
    assert!(!format!("{e:?}").contains("fake-upstream-secret"));
    let req = server.take_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/v1/app-providers/agy/one/switch");
    assert_eq!(req.body, serde_json::json!({"targetHostId":"host"}));
    server.join();
}
#[tokio::test]
async fn cc04_agy_sdk_rejects_path_identity_without_transport() {
    let client = crate::GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let e = client
        .agy_provider_switch(
            "../escape",
            &AgyTarget { target_host_id: "host".into() },
            "fake-secret-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{e:?}").contains("fake-secret-token"));
}
