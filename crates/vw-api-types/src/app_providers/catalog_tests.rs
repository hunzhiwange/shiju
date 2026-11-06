use super::catalog::*;
#[test]
fn cc04_catalog_dto_rejects_secret_fields_and_preserves_unknown_state() {
    let state = ProviderSelection::Unavailable { reason: ProviderReadUnavailable::Unreadable };
    let value = serde_json::to_value(&state).unwrap();
    assert_eq!(serde_json::from_value::<ProviderSelection>(value.clone()).unwrap(), state);
    assert_ne!(state, ProviderSelection::Unconfigured);
    let mut secret = value;
    secret["apiKey"] = "fixture-secret".into();
    assert!(serde_json::from_value::<ProviderSelection>(secret).is_err());
}
