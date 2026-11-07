use super::agy::*;
#[test]
fn cc04_agy_dto_rejects_paths_and_redacts_secrets() {
    let value = serde_json::json!({"targetHostId":"host","profile":{"name":"Card","model":"Model","apiKey":"fake-key","baseUrl":null}});
    let parsed: AgyWrite = serde_json::from_value(value.clone()).unwrap();
    assert!(!format!("{parsed:?}").contains("fake-key"));
    for key in ["root", "home", "tenant", "command"] {
        let mut v = value.clone();
        v[key] = serde_json::json!("injected");
        assert!(serde_json::from_value::<AgyWrite>(v).is_err());
    }
    let saved = serde_json::to_value(AgySaved { catalog_saved: true }).unwrap();
    assert_eq!(saved, serde_json::json!({"catalogSaved":true}));
}
