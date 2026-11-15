use serde_json::json;
use vw_api_app_identity::{AppId};

#[test]
fn cc04_validation_distinguishes_ordinary_and_proxy_headers_and_explicit_clear() {
    use super::validation::{endpoint, merge, validate};
    use vw_api_types::app_providers::validation::ProviderRequestOptions;
    let normal: ProviderRequestOptions = serde_json::from_value(
        json!({"requestHeaders":{" Authorization ":"fixture-secret","X-Test":"hello\tworld"}}),
    )
    .unwrap();
    let normalized = merge(AppId::Claude, None, &normal).unwrap();
    assert_eq!(normalized.request_headers.as_ref().unwrap()["authorization"], "fixture-secret");
    for header in [
        "host",
        "Content-Length",
        "CONTENT-TYPE",
        "Authorization",
        "x-goog-api-key",
        "traceparent",
        "X-Azure-Ref",
    ] {
        let value: ProviderRequestOptions = serde_json::from_value(
            json!({"localProxyRequestOverrides":{"headers":{header:"value"}}}),
        )
        .unwrap();
        assert!(validate(AppId::Claude, &value).is_err());
    }
    for headers in [
        json!({"X-Test":"1","x-test":"2"}),
        json!({" X-Test ":"1","x-test":"2"}),
        json!({"draft-header:fixture":"value"}),
        json!({"X-Test":"bad\u{007f}"}),
    ] {
        let options: ProviderRequestOptions =
            serde_json::from_value(json!({"requestHeaders":headers})).unwrap();
        assert!(validate(AppId::Claude, &options).is_err());
    }
    for url in [
        "https://fixture.test/a b",
        "https://fixture.test\\path",
        "http://fixture.test",
        "https://fixture.test?secret=1",
        "https://fixture.test#secret",
        "https:fixture.test",
    ] {
        assert!(endpoint(url).is_err());
    }
    for url in [
        "https://fixture.test/v1",
        "http://localhost:8080",
        "http://127.0.0.1:8080",
        "http://[::1]:8080",
    ] {
        assert!(endpoint(url).is_ok());
    }
    let old=serde_json::from_value(json!({"requestHeaders":{"x-test":"private"},"apiKeyField":"ANTHROPIC_API_KEY","localProxyRequestOverrides":{"body":{"temperature":0.2}}})).unwrap();
    let clear=serde_json::from_value(json!({"requestHeaders":{},"apiKeyField":"ANTHROPIC_AUTH_TOKEN","localProxyRequestOverrides":{}})).unwrap();
    assert_eq!(merge(AppId::Claude, Some(&old), &clear).unwrap(), clear);
    assert!(validate(AppId::Gemini, &clear).is_err());
}
