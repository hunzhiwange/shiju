use super::universal::UniversalProviderWrite;
#[test]
fn cc04_universal_dto_rejects_implicit_secrets_and_runtime_fields() {
    let valid = serde_json::json!({"config":{"name":"Fixture","providerType":"custom","apps":{},"baseUrl":"https://example.com"},"apiKey":{"action":"keep"}});
    let parsed: UniversalProviderWrite = serde_json::from_value(valid.clone()).unwrap();
    assert!(serde_json::to_value(parsed).unwrap()["apiKey"].get("value").is_none());
    for field in ["tenant", "current", "path", "meta"] {
        let mut bad = valid.clone();
        bad[field] = serde_json::json!(true);
        assert!(serde_json::from_value::<UniversalProviderWrite>(bad).is_err());
    }
    let mut bad = valid.clone();
    bad["apiKey"] = serde_json::json!("fixture-secret");
    assert!(serde_json::from_value::<UniversalProviderWrite>(bad).is_err());
    let mut bad = valid;
    bad["config"]["models"] = serde_json::json!({"codex":{"apiKey":"fixture-secret"}});
    assert!(serde_json::from_value::<UniversalProviderWrite>(bad).is_err());
}
