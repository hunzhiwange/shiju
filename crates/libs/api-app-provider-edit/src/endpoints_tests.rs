use super::endpoints::*;
#[test]
fn cc04_endpoints_dto_rejects_timestamps_secrets_and_implicit_kind() {
    for value in [
        serde_json::json!({"action":"add","url":"https://fixture.test"}),
        serde_json::json!({"action":"add","url":"https://fixture.test","kind":"baseUrl","lastUsed":1}),
        serde_json::json!({"action":"markUsed","url":"https://fixture.test","kind":"baseUrl","current":true}),
    ] {
        assert!(serde_json::from_value::<AppProviderEndpointChange>(value).is_err());
    }
    let value = serde_json::json!({"action":"markUsed","url":"https://fixture.test/v1/messages","kind":"fullUrl"});
    let request: AppProviderEndpointChange = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), value);
}
