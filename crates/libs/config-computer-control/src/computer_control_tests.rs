use crate::*;

#[test]
fn cc_control_03_defaults_deny_and_sensitive_fields_are_rejected() {
    let user = ComputerControlUserConfig::default();
    assert!(!user.native_apps && !user.chrome && !user.excel && !user.word && !user.locked_use);
    for key in ["pairing_token", "document_body", "screenshot"] {
        let value = serde_json::json!({key: "sensitive"});
        assert!(serde_json::from_value::<ComputerControlConfig>(value.clone()).is_err());
        assert!(serde_json::from_value::<ComputerControlUserConfig>(value).is_err());
    }
}
