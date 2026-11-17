use crate::host::Intent;

#[test]
fn usage_request_without_project_or_host_uses_local_context() {
    let intent: Intent = serde_json::from_str(r#"{"active":true}"#).unwrap();
    assert_eq!(intent.project_id.as_ref(), vw_api_transport::LOCAL_PROJECT_ID);
    assert_eq!(intent.target_host, vw_api_transport::LOCAL_HOST_ID);
    assert_eq!(intent.active, Some(true));
}

#[test]
fn usage_request_preserves_explicit_project_for_existing_records() {
    let intent: Intent = serde_json::from_str(
        r#"{"project_id":"existing-project","target_host":"desktop-local"}"#,
    ).unwrap();
    assert_eq!(intent.project_id.as_ref(), "existing-project");
    assert_eq!(intent.target_host, vw_api_transport::LOCAL_HOST_ID);
}
