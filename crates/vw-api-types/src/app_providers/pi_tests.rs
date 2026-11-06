use super::pi::*;
#[test]
fn cc04_pi_dto_rejects_paths_and_redacts_declarations() {
    let value = serde_json::json!({"targetHostId":"host","profile":{"name":"fixture","declaration":{"options":{"apiKey":"fake-secret"}}}});
    let request: PiWrite = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(request.activation, PiActivation::CatalogOnly);
    assert!(!format!("{request:?}").contains("fake-secret"));
    let mut bad = value;
    bad["path"] = serde_json::json!("/tmp/arbitrary");
    assert!(serde_json::from_value::<PiWrite>(bad).is_err());
}

#[test]
fn cc04_pi_dto_rejects_rename_and_conceals_usage_credentials() {
    assert!(serde_json::from_value::<PiWrite>(serde_json::json!({"targetHostId":"host","originalId":"old","profile":{"name":"display","declaration":{}}})).is_err());
    let usage:PiUsageWrite=serde_json::from_value(serde_json::json!({"targetHostId":"host","usageScript":{"enabled":true,"language":"javascript","code":"secret-code","accessToken":"fake-secret"}})).unwrap();
    assert!(!format!("{usage:?}").contains("fake-secret"));
    assert!(!format!("{:?}", usage.usage_script).contains("secret-code"));
}
