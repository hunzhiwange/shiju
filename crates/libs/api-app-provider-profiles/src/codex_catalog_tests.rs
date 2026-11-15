use super::codex_catalog::*;
use serde_json::json;
#[test]
fn cc04_codex_catalog_contract_rejects_auth_and_paths_and_redacts_debug() {
    let value = json!({"targetHostId":"host","profile":{"name":"test","config":"fake-secret-config","apiKey":"fake-secret-key"},"catalog":null});
    let request: CodexWrite = serde_json::from_value(value.clone()).unwrap();
    assert!(!format!("{request:?}").contains("fake-secret"));
    for field in ["path", "auth", "tenant", "loginOnDisk", "command"] {
        let mut invalid = value.clone();
        invalid[field] = json!("bad");
        assert!(serde_json::from_value::<CodexWrite>(invalid).is_err());
    }
    let result = CodexResult { catalog_saved: true, live: CodexLiveResult::RecoveryRequired };
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!({"catalogSaved":true,"live":"recovery_required"})
    );
}
