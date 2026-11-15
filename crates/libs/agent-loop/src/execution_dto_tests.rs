use super::*;
#[test]
fn legacy_result_dto_preserves_permission_request_and_saturates_duration_ms() {
    let permission_request = PermissionRequestDto {
        reason: "needs approval".to_string(),
        warning: Some("requires review".to_string()),
        updated_input: Some(serde_json::json!({ "value": "blocked" })),
    };

    let dto = legacy_result_dto(
        "testcov_0093_legacy",
        Some("call-legacy"),
        false,
        Some("blocked".to_string()),
        Some(permission_request.clone()),
        Duration::from_secs(u64::MAX),
    );

    let telemetry = dto.telemetry.as_ref().expect("telemetry should be present");
    assert_eq!(dto.tool_use_id.as_deref(), Some("call-legacy"));
    assert_eq!(dto.tool_id.as_ref().map(|id| id.as_ref()), Some("testcov_0093_legacy"));
    assert_eq!(dto.success, Some(false));
    assert_eq!(dto.permission_request.as_ref(), Some(&permission_request));
    assert_eq!(telemetry["attributes"]["duration_ms"], serde_json::json!(u64::MAX));
    assert_eq!(telemetry["attributes"]["duration_secs"], serde_json::json!(u64::MAX));
}
