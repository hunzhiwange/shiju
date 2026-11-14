use super::{ComputerControlApiError, test_support};
use serde_json::json;
use vw_api_types::computer_control::*;

#[tokio::test]
async fn cc_control_18_client_decodes_version_after_save() {
    let server = test_support::server(vec![(200, json!({"permission_version":2}))]);
    let response = server.client().computer_control_save(&ControlSettingsUpdate {
        permission_version: PermissionVersion(1), feature: ControlSetting::Chrome, enabled: true,
    }).await.unwrap();
    assert_eq!(response.permission_version, PermissionVersion(2));
}
#[tokio::test]
async fn cc_control_18_client_preserves_rejection() {
    let server = test_support::server(vec![(403, json!({"error":"forbidden"}))]);
    assert_eq!(server.client().computer_control_stop().await.unwrap_err(), ComputerControlApiError::Http(403));
}
