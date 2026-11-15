use super::*;

#[test]
fn hermes_api_isolates_memory_model_omits_credentials() {
    let snapshot = hermes::model_snapshot(Some(b"model:\n  default: test\n  provider: custom\n  api_key: secret\n  base_url: https://secret@example.com\n")).unwrap().unwrap();
    let json = serde_json::to_value(snapshot).unwrap();
    assert_eq!(json, serde_json::json!({"default":"test", "provider":"custom"}));
    assert_eq!(hermes::model_snapshot(None).unwrap(), None);
    assert!(hermes::model_snapshot(Some(b"model: []")).is_err());
}
