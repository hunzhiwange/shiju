use super::*;
use vw_api_app_provider_extended::hermes::*;
#[tokio::test]
async fn cc04_hermes_sdk_rejects_path_identity_without_request() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let error = client
        .hermes_provider_add(
            "../escape",
            &HermesTarget { target_host_id: "host".into() },
            "fake-token",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-token"));
}
