use super::mcode::*;
#[test]
fn cc04_mcode_dto_rejects_paths_and_redacts_declarations() {
    let value = serde_json::json!({"targetHostId":"host","profile":{"name":"fixture","declaration":{"options":{"apiKey":"fake-secret"}}}});
    let request: McodeWrite = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(request.activation, McodeActivation::CatalogOnly);
    assert!(!format!("{request:?}").contains("fake-secret"));
    let mut bad = value;
    bad["path"] = serde_json::json!("/tmp/arbitrary");
    assert!(serde_json::from_value::<McodeWrite>(bad).is_err());
}
#[test]
fn cc04_mcode_editor_dto_rejects_paths_and_conceals_yaml() {
    let value = serde_json::json!({"targetHostId":"host","name":"Fixture","yaml":"fake-secret"});
    let body: McodeEditorWrite = serde_json::from_value(value.clone()).unwrap();
    assert!(!format!("{body:?}").contains("fake-secret"));
    let mut invalid = value;
    invalid["path"] = serde_json::json!("/tmp/foreign");
    assert!(serde_json::from_value::<McodeEditorWrite>(invalid).is_err());
}
