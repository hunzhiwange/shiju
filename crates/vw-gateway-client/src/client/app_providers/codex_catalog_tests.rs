use super::*;
use vw_api_types::app_providers::codex_catalog::*;
#[tokio::test]
async fn cc04_codex_catalog_sdk_rejects_path_identity_without_request() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let error = client
        .codex_provider_switch(
            "../escape",
            &CodexTarget { target_host_id: "host".into() },
            "fake-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-token"));
}
