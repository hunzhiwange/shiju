use super::*;
use vw_api_types::app_providers::{claude_desktop::*, desktop_mapping::*};
#[tokio::test]
async fn cc04_desktop_mapping_sdk_scopes_import_and_discards_error_payload() {
    let server = super::super::test_support::server_with_tenant_scope(
        vec![
            (200, serde_json::json!({"imported":2})),
            (403, serde_json::json!({"secret":"fake-response-secret"})),
        ],
        vw_api_types::id::TenantScope::new("p", "a").unwrap(),
    );
    let client = server.client();
    let target = ClaudeDesktopTarget { target_host_id: "host".into() };
    assert_eq!(
        client.claude_desktop_import_claude(&target, "fake-token").await.unwrap().imported,
        2
    );
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/app-providers/claude-desktop/providers/import-claude");
    assert_eq!(request.body, serde_json::json!({"targetHostId":"host"}));
    let err = client.claude_desktop_default_routes(&target, "fake-token").await.unwrap_err();
    assert!(!format!("{err:?}").contains("fake-response-secret"));
    server.take_request();
    server.join();
}
#[tokio::test]
async fn cc04_desktop_mapping_sdk_rejects_path_identity() {
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
