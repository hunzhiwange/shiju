use crate::*;

#[test]
fn cc_control_03_defaults_deny_and_sensitive_fields_are_rejected() {
    let user = ComputerControlUserConfig::default();
    assert!(!user.native_apps && !user.locked_use);
    for key in ["pairing_token", "document_body", "screenshot"] {
        let value = serde_json::json!({key: "sensitive"});
        assert!(serde_json::from_value::<ComputerControlConfig>(value.clone()).is_err());
        assert!(serde_json::from_value::<ComputerControlUserConfig>(value).is_err());
    }
}

#[test]
fn removed_integrations_are_discarded_when_loading_old_profiles() {
    let user: ComputerControlUserConfig = serde_json::from_value(serde_json::json!({
        "native_apps": true,
        "locked_use": true,
        "applications": {"com.example.editor": "deny"},
        "chrome": true,
        "browser_permissions": {"defaults": {"access": "allow"}},
        "chrome_origins": {"https://example.org": "allow"},
        "excel": true,
        "excel_documents": {"workbook": "allow"},
        "word": true,
        "word_documents": {"document": "allow"}
    })).unwrap();

    assert_eq!(serde_json::to_value(user).unwrap(), serde_json::json!({
        "native_apps": true,
        "locked_use": true,
        "applications": {"com.example.editor": "deny"}
    }));
}
