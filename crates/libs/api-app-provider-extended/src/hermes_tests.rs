use super::hermes::*;
#[test]
fn cc04_hermes_dto_rejects_paths_and_redacts_declarations() {
    let value = serde_json::json!({"targetHostId":"host","profile":{"name":"fixture","declaration":{"options":{"apiKey":"fake-secret"}}}});
    let request: HermesWrite = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(request.activation, HermesActivation::CatalogOnly);
    assert!(!format!("{request:?}").contains("fake-secret"));
    let mut bad = value;
    bad["path"] = serde_json::json!("/tmp/arbitrary");
    assert!(serde_json::from_value::<HermesWrite>(bad).is_err());
}
