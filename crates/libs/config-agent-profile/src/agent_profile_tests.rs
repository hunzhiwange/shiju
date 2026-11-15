use serde_json::json;

#[test]
fn agent_profile_fixed_values_and_invalid_shapes() {
    use crate::AgentDefaults;
    let defaults: AgentDefaults = serde_json::from_value(json!({
        "skills": {"mode": "fixed", "value": ["skill-id"]},
        "mcp": {"mode": "fixed", "value": []}
    }))
    .unwrap();
    let value = serde_json::to_value(defaults).unwrap();
    assert_eq!(value["skills"]["value"], json!(["skill-id"]));
    assert_eq!(value["mcp"]["value"], json!([]));
    for invalid in [
        json!({"model": {"mode": "fixed", "value": {"provider": "p"}}}),
        json!({"model": {"mode": "fixed", "value": {"provider": "", "model": "m"}}}),
        json!({"skills": {"mode": "fixed"}}),
        json!({"skills": {"mode": "inherit", "value": []}}),
    ] {
        assert!(serde_json::from_value::<AgentDefaults>(invalid).is_err());
    }
}

#[test]
fn agent_config_cleanup_full_access_default_roundtrips() {
    use crate::AgentDefaults;
    let old: AgentDefaults = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(!old.full_access);
    for enabled in [false, true] {
        let defaults = AgentDefaults { full_access: enabled, ..Default::default() };
        let encoded = serde_json::to_value(&defaults).unwrap();
        assert_eq!(encoded["full_access"], enabled);
        assert_eq!(serde_json::from_value::<AgentDefaults>(encoded).unwrap(), defaults);
    }
}
