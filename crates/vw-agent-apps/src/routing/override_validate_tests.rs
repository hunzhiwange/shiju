use crate::providers::request_options::{update, view};
use serde_json::{Value, json};
use vw_api_types::{agent_apps::AppId, app_providers::validation::ProviderRequestOptions};

fn options(overrides: Value) -> ProviderRequestOptions {
    serde_json::from_value(json!({"localProxyRequestOverrides": overrides})).unwrap()
}

#[test]
fn cc_route_013_override_validate_expected_behavior() {
    let row = json!({"claude": {}});
    for name in [
        "Host",
        "Content-Length",
        "Transfer-Encoding",
        "Connection",
        "Proxy-Authorization",
        "Proxy-Authenticate",
        "TE",
        "Trailer",
        "Upgrade",
        "Accept-Encoding",
        "Content-Type",
        "Authorization",
        "X-Api-Key",
        "X-Goog-Api-Key",
        "ChatGPT-Account-Id",
        "Session_ID",
        "X-Client-Request-Id",
        "X-Codex-Window-Id",
        "X-Forwarded-Host",
        "X-Forwarded-Port",
        "X-Forwarded-Proto",
        "Forwarded",
        "CF-Connecting-IP",
        "CF-IPCountry",
        "CF-Ray",
        "CF-Visitor",
        "True-Client-IP",
        "Fastly-Client-IP",
        "X-Azure-ClientIP",
        "X-Azure-FDID",
        "X-Azure-Ref",
        "Akamai-Origin-Hop",
        "X-Akamai-Config-Log-Detail",
        "X-Request-Id",
        "X-Correlation-Id",
        "X-Trace-Id",
        "X-Amzn-Trace-Id",
        "X-B3-TraceId",
        "X-B3-SpanId",
        "X-B3-ParentSpanId",
        "X-B3-Sampled",
        "TraceParent",
        "TraceState",
    ] {
        let input = options(json!({"headers": {name: "fixture"}}));
        assert!(update(AppId::Claude, &row, &input).is_err(), "accepted {name}");
    }

    for headers in [
        json!({"": "fixture"}),
        json!({"X Test": "fixture"}),
        json!({"X-Test:": "fixture"}),
        json!({"X-测试": "fixture"}),
        json!({" X-Test ": "one", "x-test": "two"}),
    ] {
        assert!(update(AppId::Claude, &row, &options(json!({"headers": headers}))).is_err());
    }
    for byte in (0..=31).filter(|&byte| byte != b'\t').chain([127]) {
        let value = format!("fixture{}value", char::from(byte));
        let input = options(json!({"headers": {"X-Test": value}}));
        assert!(update(AppId::Claude, &row, &input).is_err(), "accepted byte {byte}");
    }
    for value in [json!(true), json!(false), Value::Null] {
        let input = options(json!({"body": {"stream": value}}));
        assert!(update(AppId::Claude, &row, &input).is_err());
    }
}

#[test]
fn cc_route_013_override_json_requires_objects_and_string_header_values() {
    for field in ["headers", "body"] {
        for value in [json!([]), json!(1), json!(false), json!("{}"), Value::Null] {
            assert!(
                serde_json::from_value::<ProviderRequestOptions>(json!({
                    "localProxyRequestOverrides": {field: value},
                }))
                .is_err(),
                "accepted non-object {field}"
            );
        }
    }
    for raw in [
        r#"{"localProxyRequestOverrides":{"headers":{"X-Test":1}}}"#,
        r#"{"localProxyRequestOverrides":{"headers":{"X-Test":"one","X-Test":"two"}}}"#,
    ] {
        assert!(serde_json::from_str::<ProviderRequestOptions>(raw).is_err());
    }
}

#[test]
fn cc_route_013_empty_override_members_are_omitted_on_save() {
    for (app, row) in [(AppId::Claude, json!({"claude": {}})), (AppId::Codex, json!({"codex": {}}))]
    {
        let input = options(json!({
            "headers": {" X-Test ": "fixture\tvalue"},
            "body": {},
        }));
        let stored = update(app, &row, &input).unwrap();
        assert_eq!(
            stored["requestOptions"]["localProxyRequestOverrides"],
            json!({"headers": {"x-test": "fixture\tvalue"}})
        );
        assert!(view(app, &stored).unwrap().overrides_configured);

        let input = options(json!({"headers": {}, "body": {"nested": {"stream": true}}}));
        let stored = update(app, &stored, &input).unwrap();
        assert_eq!(
            stored["requestOptions"]["localProxyRequestOverrides"],
            json!({"body": {"nested": {"stream": true}}})
        );

        let clear = options(json!({"headers": {}, "body": {}}));
        let cleared = update(app, &stored, &clear).unwrap();
        assert_eq!(cleared["requestOptions"]["localProxyRequestOverrides"], json!({}));
        assert!(!view(app, &cleared).unwrap().overrides_configured);
    }
}
