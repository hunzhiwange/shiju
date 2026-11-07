use vw_api_types::{agent_apps::AppId, app_providers::mode_apply::*};
#[tokio::test]
async fn cc04_mode_apply_sdk_sends_typed_operation_and_discards_error_payload() {
    let server = super::super::test_support::server_with_tenant_scope(
        vec![
            (
                200,
                serde_json::json!({"configurationChanged":true,"attached":true,"targetPublication":"published"}),
            ),
            (503, serde_json::json!({"secret":"fake-sensitive"})),
        ],
        vw_api_types::id::TenantScope::new("p", "c").unwrap(),
    );
    let request = ModeApplyRequest {
        target_host_id: "host".into(),
        action: ModeApplyAction::Proxy { provider_id: "route".into() },
    };
    let result =
        server.client().provider_mode_apply(AppId::Claude, &request, "fake-token").await.unwrap();
    assert!(result.attached);
    let observed = server.take_request();
    assert_eq!(observed.method, "POST");
    assert_eq!(observed.path, "/v1/app-providers/claude/mode/apply");
    let error = server
        .client()
        .provider_mode_apply(AppId::Claude, &request, "fake-token")
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-sensitive"));
    server.take_request();
    server.join();
}
