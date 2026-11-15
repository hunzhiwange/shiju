use super::env_clean::*;
#[test]
fn cc04_env_clean_dto_accepts_receipts_and_rejects_file_coordinates() {
    let value = serde_json::json!({"targetHostId":"host", "selections":[{"diagnosticId":"opaque", "baseDigest":"base"}]});
    let request: EnvCleanRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), value);
    for field in ["path", "line", "value", "tenant", "cleanupAvailable"] {
        let mut invalid = value.clone();
        invalid["selections"][0][field] = serde_json::json!("injected");
        assert!(serde_json::from_value::<EnvCleanRequest>(invalid).is_err());
    }
}
