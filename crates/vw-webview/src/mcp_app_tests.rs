use super::{WRAPPER_CSP, app_csp, frame_origin_allowed};
use serde_json::json;

use vw_api_types::mcp_app_network::McpAppNetworkPolicy;

#[test]
fn mcp_app_network_declared_frame_sources_are_scoped() {
    let policy = json!({
        "resourceDomains": ["https://cdn.example.test"],
        "connectDomains": ["https://api.example.test"],
        "frameDomains": ["https://frame.example.test:8443"],
        "baseUriDomains": ["https://base.example.test"],
        "redirectDomains": ["https://links.example.test"]
    });
    let policy = McpAppNetworkPolicy::parse(Some(&policy)).unwrap();
    let document = app_csp(&policy);
    assert!(WRAPPER_CSP.contains("frame-src 'self';"));
    assert!(frame_origin_allowed(&policy, "https://frame.example.test:8443/video"));
    assert!(!frame_origin_allowed(&policy, "https://frame.example.test/video"));
    assert!(!frame_origin_allowed(&policy, "https://cdn.example.test"));
    assert!(!frame_origin_allowed(&policy, "https://links.example.test"));
    assert!(document.contains("frame-src https://frame.example.test:8443;"));
    assert!(document.contains("connect-src https://api.example.test;"));
    assert!(document.contains("img-src data: https://cdn.example.test;"));
    assert!(document.contains("media-src https://cdn.example.test;"));
    assert!(document.contains("base-uri https://base.example.test;"));
    assert!(!document.contains("links.example.test"));
}

#[test]
fn mcp_app_network_rejects_invalid_declarations() {
    for origin in [
        "http://host.example.test",
        "https://*.example.test",
        "https://host.example.test:0",
        "https://host.example.test:65536",
        "https://host.example.test/+",
        "https://host.example.test?token=test",
        "https://host.example.test; frame-src https:",
        "https://user@host.example.test",
        "https://host..example.test",
        "https://host.example.test:+443",
        "https://127.1",
    ] {
        assert!(
            McpAppNetworkPolicy::parse(Some(&json!({"connectDomains":[origin]}))).is_err(),
            "{origin}"
        );
    }
    assert!(McpAppNetworkPolicy::parse(Some(&json!({"unsupported":[]}))).is_err());
    assert!(McpAppNetworkPolicy::parse(Some(&json!({"resourceDomains":"https:"}))).is_err());
}

#[test]
fn mcp_app_network_defaults_deny_and_origins_roundtrip() {
    let csp = app_csp(&McpAppNetworkPolicy::default());
    for directive in ["connect-src", "frame-src", "base-uri", "form-action", "object-src"] {
        assert!(csp.contains(&format!("{directive} 'none'")));
    }
    let raw = json!({
        "connectDomains": ["https://API.example.test:443"],
        "redirectDomains": ["https://links.example.test:8443"]
    });
    let serialized = serde_json::to_value(McpAppNetworkPolicy::parse(Some(&raw)).unwrap()).unwrap();
    assert_eq!(serialized["connectDomains"], json!(["https://api.example.test"]));
    assert_eq!(serialized["redirectDomains"], raw["redirectDomains"]);
    assert!(
        !app_csp(&McpAppNetworkPolicy::parse(Some(&serialized)).unwrap())
            .contains("links.example.test")
    );
}
