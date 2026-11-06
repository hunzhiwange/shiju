use super::claude::*;
use serde_json::json;
#[test]
fn cc04_claude_official_needs_no_fake_secret_and_roots_are_rejected() {
    let mut body =
        json!({"targetHostId":"host","claude":{"name":"Official","config":{"kind":"official"}}});
    let request: ClaudeWrite = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(request.activation, ClaudeActivation::CatalogOnly);
    assert!(!format!("{request:?}").contains("Official"));
    body["home"] = json!("/tmp/unsafe");
    assert!(serde_json::from_value::<ClaudeWrite>(body).is_err());
    assert!(
        serde_json::from_value::<ClaudeConfig>(json!({"kind":"official","api_key":"secret"}))
            .is_err()
    );
}
