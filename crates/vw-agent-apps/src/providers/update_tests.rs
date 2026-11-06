use super::update::update;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::ProviderSecretWrite, update::AppProviderUpdate},
};
fn stored() -> serde_json::Value {
    serde_json::json!({"name":"fixture", "protocol":"anthropic", "apiKey":"fixture-secret", "baseUrl":"https://fixture.example.test", "model":"fixture-model", "meta":{"notes":"keep", "sortIndex":7}, "createdAt":123})
}
#[test]
fn cc04_update_preserves_unedited_fields() {
    let root = tempfile::tempdir().unwrap();
    let record = root.path().join("record.json");
    std::fs::write(&record, serde_json::to_vec(&stored()).unwrap()).unwrap();
    let before: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    let request = AppProviderUpdate { name: Some("edited".into()), ..Default::default() };
    let after = update(AppId::Claude, "fixture", &before, &request).unwrap();
    std::fs::write(&record, serde_json::to_vec(&after).unwrap()).unwrap();
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(record).unwrap()).unwrap();
    let mut expected = before;
    expected["name"] = "edited".into();
    assert_eq!(actual, expected);
}
#[test]
fn cc04_update_redacted_secret_is_unchanged() {
    let before = stored();
    for marker in ["[redacted]", "<redacted>", "********", "••••••••"] {
        let request = AppProviderUpdate {
            api_key: ProviderSecretWrite::Replace { value: marker.into() },
            ..Default::default()
        };
        assert!(
            update(AppId::Claude, "fixture", &before, &request).is_err(),
            "placeholder must not replace stored credential"
        );
    }
    let kept = update(AppId::Claude, "fixture", &before, &AppProviderUpdate::default()).unwrap();
    assert_eq!(kept, before);
}
#[test]
fn cc04_update_explicit_secret_and_invalid_profile_are_validated() {
    let mut request = AppProviderUpdate {
        api_key: ProviderSecretWrite::Replace { value: "fixture-rotated".into() },
        ..Default::default()
    };
    let actual = update(AppId::Claude, "fixture", &stored(), &request).unwrap();
    assert_eq!(actual["apiKey"], "fixture-rotated");
    request.api_key = ProviderSecretWrite::Clear;
    assert!(update(AppId::Claude, "fixture", &stored(), &request).is_err());
    request.api_key = ProviderSecretWrite::Keep;
    request.base_url = Some("https://user:password@example.test".into());
    assert!(update(AppId::Claude, "fixture", &stored(), &request).is_err());
    assert!(update(AppId::Claude, "../invalid", &stored(), &request).is_err());
}
