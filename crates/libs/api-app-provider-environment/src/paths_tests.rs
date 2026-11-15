use super::paths::*;
#[test]
fn cc04_paths_dto_rejects_client_home_and_write_payloads() {
    for extra in ["home", "tenant", "profile", "content", "environment"] {
        let mut value = serde_json::json!({"targetHostId":"host"});
        value[extra] = serde_json::json!("unauthorized");
        assert!(serde_json::from_value::<ProviderPathsRequest>(value).is_err());
    }
    let value: ProviderPathsRequest = serde_json::from_value(
        serde_json::json!({"targetHostId":"host","directoryOverride":"~/custom"}),
    )
    .unwrap();
    assert_eq!(value.directory_override.as_deref(), Some("~/custom"));
    assert_eq!(serde_json::to_value(ConfigPathState::Inaccessible).unwrap(), "inaccessible");
}
