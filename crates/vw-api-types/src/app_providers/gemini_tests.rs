use super::gemini::*;
use serde_json::json;
#[test]
fn cc04_gemini_dto_rejects_paths_and_conceals_credentials() {
    let value = json!({"targetHostId":"host","profile":{"name":"Fixture","config":{"kind":"third_party","base_url":"https://fixture.test","api_key":"fake-secret"}},"activation":"switch"});
    let request: GeminiWrite = serde_json::from_value(value.clone()).unwrap();
    assert!(!format!("{request:?}").contains("fake-secret"));
    let mut bad = value.clone();
    bad["path"] = json!("/tmp/escape");
    assert!(serde_json::from_value::<GeminiWrite>(bad).is_err());
    let mut bad = value;
    bad["profile"]["config"] = json!({"kind":"official","api_key":"fake"});
    assert!(serde_json::from_value::<GeminiWrite>(bad).is_err());
}
