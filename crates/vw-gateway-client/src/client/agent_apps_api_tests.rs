use serde_json::json;
use vw_api_types::{agent_apps::CustomIconEntry, id::TenantScope};

use super::{GatewayClient, test_support};
use crate::endpoint::GatewayAuth;

#[tokio::test]
async fn cc_base_icon_sdk_read_preserves_headers_and_chinese_icons() {
    let icon = CustomIconEntry {
        id: "icon-1".into(),
        name: "工作图标".into(),
        data: "data:image/png;base64,aGVsbG8=".into(),
        created_at: 1730000000123,
    };
    let server = test_support::server_with_tenant_scope(
        vec![(200, json!([icon])), (200, json!([]))],
        TenantScope::new("platform-a", "company-b").unwrap(),
    );
    let endpoint = server
        .client()
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some("test-icon-key".into()) });
    let client = GatewayClient::new(endpoint).unwrap();
    let icons: Vec<CustomIconEntry> = client.custom_icons_list().await.unwrap();
    assert_eq!(icons, vec![icon]);
    // Re-read after a disconnected HTTP connection observes the current list.
    assert!(client.custom_icons_list().await.unwrap().is_empty());
    for _ in 0..2 {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/agent-apps/custom-icons");
        assert_eq!(request.header("authorization"), Some("Bearer test-icon-key"));
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-b"));
        assert_eq!(request.body, serde_json::Value::Null);
    }
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_read_surfaces_business_errors() {
    let server = test_support::server(vec![(
        503,
        json!({
            "code": "storage_unavailable", "message": "图标存储暂不可用"
        }),
    )]);
    let error = server.client().custom_icons_list().await.unwrap_err();
    assert!(error.contains("503"), "{error}");
    assert!(error.contains("storage_unavailable"), "{error}");
    assert!(error.contains("图标存储暂不可用"), "{error}");
    server.join();
}

#[tokio::test]
async fn cc_base_icon_sdk_read_rejects_invalid_success_payload() {
    let server = test_support::server(vec![(200, json!([{"id": "incomplete"}]))]);
    assert!(server.client().custom_icons_list().await.is_err());
    server.join();
}
