use super::openclaw_defaults::*;
use serde_json::json;

#[test]
fn openclaw_defaults_projects_fields_preserves_extensions_and_timeout_priority() {
    let pre = br#"{ /* root */ "agents": {"other": true, "defaults": {"workspace":"~/work", "timeoutSeconds": 5, "timeout":9, "contextTokens":200, "maxConcurrent":2, "model":{"primary":"a"}, "future":{"x":true}}}, "env": {"KEEP":"yes"}}"#;
    let defaults = get_agents_defaults(Some(pre)).unwrap().unwrap();
    assert_eq!(defaults.form().timeout_seconds, "5");
    let mut form = defaults.form();
    form.workspace = "  ~/new  ".into();
    form.timeout_seconds = " 6.5 ".into();
    form.context_tokens.clear();
    let updated = defaults.with_form(&form).unwrap();
    let next = prepare_agents_defaults(Some(pre), &updated).unwrap();
    assert!(String::from_utf8_lossy(&next).contains("/* root */"));
    let actual = get_agents_defaults(Some(&next)).unwrap().unwrap();
    assert_eq!(actual.0["workspace"], "~/new");
    assert_eq!(actual.0["timeoutSeconds"], 6.5);
    assert!(!actual.0.contains_key("timeout"));
    assert!(!actual.0.contains_key("contextTokens"));
    assert_eq!(actual.0["future"], json!({"x":true}));
    assert_eq!(actual.0["model"], json!({"primary":"a"}));
    let root: serde_json::Value = crate::providers::live::patch_json::object_value(&next).unwrap();
    assert_eq!(root["env"], json!({"KEEP":"yes"}));
    assert_eq!(root["agents"]["other"], true);
}

#[test]
fn openclaw_defaults_projects_fields_legacy_clear_and_finite_validation() {
    let defaults = get_agents_defaults(Some(br#"{"agents":{"defaults":{"timeout":30}}}"#)).unwrap().unwrap();
    assert_eq!(defaults.form().timeout_seconds, "30");
    let next = prepare_agents_defaults(None, &defaults).unwrap();
    let migrated = get_agents_defaults(Some(&next)).unwrap().unwrap();
    assert_eq!(migrated.0["timeoutSeconds"], 30);
    assert!(!migrated.0.contains_key("timeout"));
    let empty = defaults.with_form(&OpenClawDefaultsForm::default()).unwrap();
    assert!(empty.0.is_empty());
    for invalid in ["NaN", "inf", "-inf", "1e999", "abc"] {
        let form = OpenClawDefaultsForm { max_concurrent: invalid.into(), ..Default::default() };
        assert!(defaults.with_form(&form).is_err());
    }
    assert_eq!(defaults.0["timeout"], 30);
}

#[test]
fn openclaw_defaults_projects_fields_missing_and_malformed_remain_distinct() {
    assert!(get_agents_defaults(None).unwrap().is_none());
    for pre in [b"".as_slice(), b"[]", br#"{"agents":null}"#, br#"{"agents":{"defaults":[]}}"#, b"{bare:'unsupported'}"] {
        assert!(get_agents_defaults(Some(pre)).is_err());
    }
    let pre = br#"{"agents":{"defaults":{"timeout":"unknown","extension":null}}}"#;
    let defaults = get_agents_defaults(Some(pre)).unwrap().unwrap();
    let next = prepare_agents_defaults(Some(pre), &defaults).unwrap();
    assert!(next == pre);
}
