use super::env_restore::*;
#[test]
fn cc04_env_restore_dto_rejects_coordinates_and_roundtrips_results() {
    let value = serde_json::json!({"targetHostId":"host", "backupId":"opaque"});
    let request: EnvRestoreRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), value);
    for field in ["path", "backupPath", "scope", "tenant", "original", "file", "force"] {
        let mut invalid = value.clone();
        invalid[field] = serde_json::json!("injected");
        assert!(serde_json::from_value::<EnvRestoreRequest>(invalid).is_err());
    }
    let result = serde_json::json!({"items":[{"file":"zshrc","status":"restored"}]});
    let parsed: EnvRestoreResult = serde_json::from_value(result.clone()).unwrap();
    assert_eq!(parsed.items[0].status, EnvRestoreStatus::Restored);
    assert_eq!(serde_json::to_value(parsed).unwrap(), result);
}
