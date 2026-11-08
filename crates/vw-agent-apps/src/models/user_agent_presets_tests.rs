use super::user_agent_presets::{USER_AGENT_PRESETS, custom_user_agent};
use std::collections::BTreeSet;

#[test]
fn catalog_user_agent_does_not_apply_without_selection() {
    for value in [None, Some(""), Some(" \t ")] {
        assert_eq!(custom_user_agent(value).unwrap(), None);
    }
    assert!(custom_user_agent(Some("agent\r\nX-Injected: yes")).is_err());
}

#[test]
fn catalog_user_agent_lists_five_selectable_presets() {
    assert_eq!(USER_AGENT_PRESETS.len(), 5);
    let mut unique = BTreeSet::new();
    for &preset in USER_AGENT_PRESETS {
        assert!(unique.insert(preset));
        let selected = custom_user_agent(Some(preset)).unwrap().unwrap();
        assert_eq!(selected.as_bytes(), preset.as_bytes());
    }
}

#[test]
fn catalog_user_agent_accepts_explicit_custom_value() {
    let selected = custom_user_agent(Some(" \tfixture/1.0\t中文 \t")).unwrap().unwrap();
    assert_eq!(selected.as_bytes(), "fixture/1.0\t中文".as_bytes());
}

#[test]
fn catalog_user_agent_rejects_invalid_provider_header_values() {
    for value in ["\n", "fixture\r", "fixture\nagent", "fixture\0agent", "fixture\x7f"] {
        assert!(custom_user_agent(Some(value)).is_err());
    }
    assert!(custom_user_agent(Some(&"a".repeat(8193))).is_err());
}
