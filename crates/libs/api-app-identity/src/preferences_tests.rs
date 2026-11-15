use super::*;
#[test]
fn cc_apps_preferences_normalization_dto_roundtrip() {
    let host = AppHostPreferences::default();
    let value = serde_json::to_value(&host).unwrap();
    assert_eq!(serde_json::from_value::<AppHostPreferences>(value).unwrap(), host);
    assert!(serde_json::from_str::<AppHostPreferences>(r#"{"command":"restart"}"#).is_err());
    assert!(serde_json::from_str::<AppUserPreferences>(r#"{"visibleApps":["unknown"]}"#).is_err());
}
