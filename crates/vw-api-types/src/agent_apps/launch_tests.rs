use super::*;
#[test]
fn cc_apps_launch_request_validation_wire_defaults_and_rejects_commands() {
    let value = serde_json::json!({"appId":"codex", "installationId":"observed", "cwd":"/project"});
    let request: LaunchRequest = serde_json::from_value(value.clone()).unwrap();
    assert!(!request.skip_permissions);
    assert!(request.args.is_empty());
    assert_eq!(
        serde_json::from_value::<LaunchRequest>(serde_json::to_value(&request).unwrap()).unwrap(),
        request
    );
    for field in ["program", "command", "credentials", "targetHostId"] {
        let mut invalid = value.clone();
        invalid[field] = "untrusted".into();
        assert!(serde_json::from_value::<LaunchRequest>(invalid).is_err());
    }
}
