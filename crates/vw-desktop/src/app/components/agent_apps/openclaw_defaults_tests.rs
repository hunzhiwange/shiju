use super::*;

#[test]
fn openclaw_defaults_editor_preserves_fields() {
    let original: OpenClawAgentsDefaults = serde_json::from_value(serde_json::json!({
        "workspace":"old", "timeout": 10, "model":{"primary":"p/m"},
        "future":{"keep":true}, "contextTokens":100
    }))
    .unwrap();
    let mut form = project_fields(&original);
    assert_eq!(form[1], "10");
    form[0] = " ~/工作区 ".into();
    form[1] = "2.5".into();
    form[2].clear();
    let value = candidate(&original, &form).ok().unwrap();
    assert_eq!(value.0["model"], original.0["model"]);
    assert_eq!(value.0["future"], original.0["future"]);
    assert_eq!(value.0["workspace"], "~/工作区");
    assert_eq!(value.0["timeoutSeconds"], 2.5);
    assert!(!value.0.contains_key("contextTokens"));
    assert!(!value.0.contains_key("timeout"));
}

#[test]
fn openclaw_defaults_editor_preserves_fields_rejects_invalid_numbers() {
    for invalid in ["NaN", "inf", "1e999", "无效"] {
        let form = [String::new(), invalid.into(), String::new(), String::new()];
        let errors = candidate(&OpenClawAgentsDefaults::default(), &form).err().unwrap();
        assert!(errors[1].is_some());
        assert!(errors[0].is_none());
    }
}
