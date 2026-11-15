use super::*;
use vw_api_types::app_providers::omp::*;
#[tokio::test]
async fn cc04_omp_sdk_rejects_path_identity_without_request() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    let error = client
        .omp_provider_add("../escape", &OmpTarget { target_host_id: "host".into() }, "fake-token")
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("fake-token"));
}
