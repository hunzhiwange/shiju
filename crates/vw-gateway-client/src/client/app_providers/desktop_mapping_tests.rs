use super::*;
use vw_api_app_provider_profiles::claude_desktop::*;
use vw_api_app_provider_profiles::desktop_mapping::*;
#[tokio::test]
async fn cc04_desktop_mapping_facade_rejects_path_identity() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    assert!(
        client
            .claude_desktop_mapping_save(
                "../escape",
                &DesktopMappingWrite {
                    target_host_id: "host".into(),
                    mapping: DesktopMapping {
                        mode: DesktopMappingMode::Proxy,
                        routes: Default::default()
                    }
                },
                "fake-token"
            )
            .await
            .is_err()
    );
}
