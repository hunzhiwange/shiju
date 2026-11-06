use super::claude::*;
use serde_json::json;
#[test]
fn cc04_claude_login_binding_is_strict_and_never_accepts_material_or_paths() {
    let good = json!({"kind":"official","account":{"accountId":"registered"}});
    let config: ClaudeConfig = serde_json::from_value(good.clone()).unwrap();
    assert_eq!(serde_json::to_value(&config).unwrap(), good);
    assert!(!format!("{config:?}").contains("registered"));
    for field in ["path", "credential", "home", "tenant"] {
        let mut bad = good.clone();
        bad["account"][field] = json!("forbidden");
        assert!(serde_json::from_value::<ClaudeConfig>(bad).is_err());
    }
    assert!(serde_json::from_value::<ClaudeConfig>(json!({"kind":"official"})).is_ok());
}
