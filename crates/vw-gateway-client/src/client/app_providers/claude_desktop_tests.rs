use super::*;
use vw_api_types::app_providers::claude_desktop::*;
#[tokio::test]
async fn cc04_claude_desktop_facade_rejects_path_identity_without_request() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let error = client
        .claude_desktop_provider_switch(
            "../escape",
            &ClaudeDesktopTarget { target_host_id: "host".into() },
            "fake-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-token"));
}
