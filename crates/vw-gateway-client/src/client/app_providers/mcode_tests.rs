use super::*;
use vw_api_app_provider_terminal::mcode::*;

#[tokio::test]
async fn cc04_mcode_sdk_rejects_path_identity_without_request() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let error = client
        .mcode_provider_add(
            "../escape",
            &McodeTarget { target_host_id: "host".into() },
            "fake-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-token"));
}
