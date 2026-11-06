use super::claude;
use serde_json::{Value, json};
#[test]
fn cc04_claude_switch_cleans_previous_provider_keys() {
    let old = json!({"env":{"CLAUDE_CODE_USE_VERTEX":"1","GOOGLE_APPLICATION_CREDENTIALS":"/fake/key.json","AWS_SECRET_ACCESS_KEY":"fake","ANTHROPIC_MODEL":"old"}});
    let output =
        claude::patch(Some(old.to_string().as_bytes()), Some(&old), &json!({"env":{}})).unwrap();
    let after: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(after["env"], json!({}));
}
#[test]
fn cc04_claude_switch_preserves_user_fields() {
    let before = br#"{ "hooks": {"x":true}, "env": {"USER_FLAG":"yes","ANTHROPIC_API_KEY":"old"}}"#;
    let after =
        claude::patch(Some(before), None, &json!({"env":{"ANTHROPIC_AUTH_TOKEN":"new"}})).unwrap();
    let value: Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(value["env"]["ANTHROPIC_AUTH_TOKEN"], "new");
    assert_eq!(value["env"]["USER_FLAG"], "yes");
    assert!(std::str::from_utf8(&after).unwrap().contains("\"hooks\": {\"x\":true}"));
}
#[test]
fn cc04_claude_outgoing_exclusives_are_value_guarded_and_legacy_bedrock_is_normalized() {
    let previous = json!({"env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"1","ENABLE_TOOL_SEARCH":"0"}});
    let live = json!({"env":{"CLAUDE_CODE_DISABLE_ARTIFACT":"1","ENABLE_TOOL_SEARCH":"user","CLAUDE_CODE_USE_NATIVE_FILE_SEARCH":"1"}});
    let target = json!({"apiKey":"fake-bedrock","env":{"CLAUDE_CODE_USE_BEDROCK":"1","AWS_REGION":"us-west-2"}});
    let after: Value = serde_json::from_slice(
        &claude::patch(Some(live.to_string().as_bytes()), Some(&previous), &target).unwrap(),
    )
    .unwrap();
    assert!(after["env"].get("CLAUDE_CODE_DISABLE_ARTIFACT").is_none());
    assert_eq!(after["env"]["ENABLE_TOOL_SEARCH"], "user");
    assert_eq!(after["env"]["CLAUDE_CODE_USE_NATIVE_FILE_SEARCH"], "1");
    assert_eq!(after["env"]["AWS_BEARER_TOKEN_BEDROCK"], "fake-bedrock");
    assert!(after.get("apiKey").is_none());
}
#[test]
fn cc04_claude_import_rejects_unrepresented_auth_instead_of_silently_dropping_it() {
    assert!(claude::import("bad",br#"{"env":{"CLAUDE_CODE_USE_VERTEX":"1","CLAUDE_CODE_USE_BEDROCK":"1","AWS_REGION":"us-west-2"}}"#).is_err());
    assert!(claude::import("bad", br#"{"apiKeyHelper":"command","env":{}}"#).is_err());
    assert!(claude::import("official", br#"{"hooks":{},"env":{}}"#).is_ok());
}
#[test]
fn cc04_claude_switch_preserves_comments_and_rejects_duplicate_credentials() {
    let bytes = br#"{ // user comment
        "hooks": {"user": true}, "env": {"ANTHROPIC_AUTH_TOKEN":"fake-old",},
    }"#;
    let result = claude::patch(Some(bytes), None, &json!({"env":{}})).unwrap();
    let text = std::str::from_utf8(&result).unwrap();
    assert!(text.contains("// user comment"));
    assert!(text.contains("\"hooks\": {\"user\": true}"));
    assert!(!text.contains("fake-old"));
    assert!(
        claude::patch(
            Some(br#"{"env":{"ANTHROPIC_API_KEY":"one","ANTHROPIC_API_KEY":"two"}}"#),
            None,
            &json!({})
        )
        .is_err()
    );
}
