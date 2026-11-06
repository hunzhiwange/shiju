use super::super::test_support;
use vw_api_types::{app_providers::claude::*, id::TenantScope};
#[tokio::test]
async fn cc04_claude_sdk_uses_native_route_and_discards_secret_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"concealed-error"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let error = server
        .client()
        .claude_provider_switch(
            "saved",
            &ClaudeTarget { target_host_id: "host".into() },
            "fake-credential-for-claude-request",
        )
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("concealed-error"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/app-providers/claude/saved/switch");
    assert_eq!(request.body, serde_json::json!({"targetHostId":"host"}));
    server.join();
}
