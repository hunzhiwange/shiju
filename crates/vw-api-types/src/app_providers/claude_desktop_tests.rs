use super::claude_desktop::*;
use serde_json::json;
#[test]
fn cc04_claude_desktop_dto_rejects_paths_and_official_payload_and_conceals_secrets() {
    let value = json!({"targetHostId":"host","profile":{"name":"Fixture","config":{"kind":"third_party","baseUrl":"https://fixture.test","authToken":"fake-secret"}},"activation":"switch"});
    let request: ClaudeDesktopWrite = serde_json::from_value(value.clone()).unwrap();
    assert!(!format!("{request:?}").contains("fake-secret"));
    let mut bad = value.clone();
    bad["path"] = json!("/tmp/escape");
    assert!(serde_json::from_value::<ClaudeDesktopWrite>(bad).is_err());
    let mut bad = value;
    bad["profile"]["config"] = json!({"kind":"official","toml":"secret"});
    assert!(serde_json::from_value::<ClaudeDesktopWrite>(bad).is_err());
}
