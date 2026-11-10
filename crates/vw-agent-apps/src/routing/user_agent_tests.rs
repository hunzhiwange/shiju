use crate::providers::request_options;
use serde_json::json;
use vw_api_types::{agent_apps::AppId, app_providers::validation::ProviderRequestOptions};

fn options(value: &str) -> ProviderRequestOptions {
    serde_json::from_value(json!({
        "localProxyRequestOverrides": {"headers": {" User-Agent ": value}},
    }))
    .unwrap()
}

#[test]
fn cc_route_016_user_agent_expected_behavior() {
    let row = json!({"codex": {}});
    for blank in ["", " \t ", "\u{3000}"] {
        let stored = request_options::update(AppId::Codex, &row, &options(blank)).unwrap();
        let view = request_options::view(AppId::Codex, &stored).unwrap();
        assert!(!view.overrides_configured, "blank User-Agent must be unset");
        assert_eq!(stored["requestOptions"]["localProxyRequestOverrides"], json!({}));

        let legacy = json!({"codex": {}, "requestOptions": options(blank)});
        assert!(!request_options::view(AppId::Codex, &legacy).unwrap().overrides_configured);
    }

    let ua = " \tfixture/1.0\t中文\u{3000}";
    let stored = request_options::update(AppId::Codex, &row, &options(ua)).unwrap();
    assert!(request_options::view(AppId::Codex, &stored).unwrap().overrides_configured);

    for invalid in ["private\nfixture", "private\r\nfixture", "\0private", "private\x7f", "\n"] {
        let error = request_options::update(AppId::Codex, &row, &options(invalid)).unwrap_err();
        assert!(!error.contains("private"));
    }
}
