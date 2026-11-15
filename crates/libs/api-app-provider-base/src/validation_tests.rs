use super::{AppProviderWrite, validation::ProviderRequestOptions};
use serde_json::json;
#[test]
fn cc04_validation_dto_is_typed_compatible_and_redacted() {
    let legacy = json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-secret","baseUrl":"https://fixture.test","model":"fixture"});
    let old: AppProviderWrite = serde_json::from_value(legacy.clone()).unwrap();
    assert!(old.request_options.is_none());
    assert_eq!(serde_json::to_value(old).unwrap(), legacy);
    let value:ProviderRequestOptions=serde_json::from_value(json!({"requestHeaders":{"Authorization":"fixture-header-secret"},"apiKeyField":"ANTHROPIC_API_KEY","localProxyRequestOverrides":{"body":{"private":"fixture-body-secret"}}})).unwrap();
    assert!(!format!("{value:?}").contains("fixture-"));
    assert!(!format!("{:?}", value.local_proxy_request_overrides).contains("fixture-"));
    for invalid in [
        json!({"apiKeyField":"ANY_ENV"}),
        json!({"requestHeaders":{"X":1}}),
        json!({"localProxyRequestOverrides":{"body":[]}}),
        json!({"unknown":true}),
        json!({"requestHeaders":null}),
    ] {
        assert!(serde_json::from_value::<ProviderRequestOptions>(invalid).is_err());
    }
}

#[test]
fn cc04_validation_dto_rejects_duplicate_headers_and_null_options() {
    use super::update::AppProviderUpdate;
    assert!(
        serde_json::from_str::<ProviderRequestOptions>(
            r#"{"requestHeaders":{"X-Test":"a","X-Test":"b"}}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<ProviderRequestOptions>(
            r#"{"localProxyRequestOverrides":{"headers":{"X-Test":"a","X-Test":"b"}}}"#
        )
        .is_err()
    );
    assert!(serde_json::from_value::<AppProviderUpdate>(json!({"requestOptions":null})).is_err());
}
