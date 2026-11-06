use super::kimi::*;
#[test]
fn cc04_kimi_dto_rejects_paths_and_redacts_declarations() {
    let value = serde_json::json!({"targetHostId":"host","profile":{"name":"fixture","declaration":{"options":{"apiKey":"fake-secret"}}}});
    let request: KimiWrite = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(request.activation, KimiActivation::CatalogOnly);
    assert!(!format!("{request:?}").contains("fake-secret"));
    let mut bad = value;
    bad["path"] = serde_json::json!("/tmp/arbitrary");
    assert!(serde_json::from_value::<KimiWrite>(bad).is_err());
}
#[test]
fn cc04_kimi_editor_dto_rejects_paths_and_conceals_toml() {
    let value = serde_json::json!({"targetHostId":"host","name":"Fixture","toml":"fake-secret"});
    let body: KimiEditorWrite = serde_json::from_value(value.clone()).unwrap();
    assert!(!format!("{body:?}").contains("fake-secret"));
    let mut invalid = value;
    invalid["path"] = serde_json::json!("/tmp/foreign");
    assert!(serde_json::from_value::<KimiEditorWrite>(invalid).is_err());
}
