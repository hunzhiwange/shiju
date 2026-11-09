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
    assert!(frame_origin_allowed(&policy.frame_domains, "https://frame.example.test:8443/video"));
    assert!(!frame_origin_allowed(&policy.frame_domains, "https://frame.example.test/video"));
    assert!(!frame_origin_allowed(&policy.frame_domains, "https://cdn.example.test"));
    assert!(!frame_origin_allowed(&policy.frame_domains, "https://links.example.test"));
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

#[test]
fn mcp_app_form_csp_and_sandbox_use_connect_origins_only() {
    use super::{form::allowed_origins, render_wrapper, wrapper_csp};

    let policy = McpAppNetworkPolicy::parse(Some(&json!({
        "connectDomains": ["https://api.example.test:8443", "https://frame.example.test"],
        "frameDomains": ["https://frame.example.test"],
        "redirectDomains": ["https://other.example.test"],
        "resourceDomains": ["https://asset.example.test"]
    })))
    .unwrap();

    let origins = allowed_origins(&policy);
    if cfg!(any(target_os = "windows", coverage)) {
        assert!(origins.is_empty());
        assert!(app_csp(&policy).contains("form-action 'none'"));
        assert!(!render_wrapper("host", "app", false).contains("allow-forms"));
        return;
    }
    assert_eq!(origins.contains(&"https://frame.example.test".into()), cfg!(target_os = "macos"));
    assert!(app_csp(&policy).contains("form-action https://api.example.test:8443"));
    let wrapper = wrapper_csp(&origins);
    assert!(wrapper.contains("frame-src 'self' https://api.example.test:8443"));
    assert!(!wrapper.contains("other.example.test"));
    assert!(!wrapper.contains("asset.example.test"));
    assert!(wrapper.contains("form-action 'none'"));
    let html = render_wrapper("host", "app", true);
    assert!(html.contains("sandbox=\"allow-scripts allow-forms\""));
    assert!(!html.contains("allow-popups"));
    assert!(!html.contains("allow-top-navigation"));
    assert!(!html.contains("allow-same-origin"));
}

#[cfg(all(unix, not(coverage)))]
#[test]
fn mcp_app_form_navigation_requires_exact_origin_and_live_document() {
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use super::{
        APP_URL,
        form::{FormNavigation, FormPolicy},
    };

    let origin = "https://api.example.test:8443";
    let policy = FormPolicy {
        parent: None,
        frame_domains: Vec::new(),
        origins: vec![origin.into(), "https://other.example.test".into()],
        active: Arc::new(AtomicBool::new(true)),
        submitted: Rc::new(Cell::new(false)),
        pending: Rc::new(Cell::new(false)),
    };
    let destination = format!("{origin}/submit");
    assert_eq!(policy.decide(APP_URL, false, false, false), FormNavigation::Allow);
    assert_eq!(
        policy.decide(&destination, true, false, true),
        FormNavigation::Confirm(origin.into())
    );
    for target in [
        "https://api.example.test/submit",
        "http://api.example.test:8443/submit",
        "https://user@api.example.test:8443/submit",
        "https://api.example.test:8443.evil.test/submit",
    ] {
        assert_eq!(policy.decide(target, true, false, true), FormNavigation::Deny);
    }
    assert_eq!(policy.decide(&destination, false, false, true), FormNavigation::Deny);
    assert_eq!(policy.decide(&destination, true, false, false), FormNavigation::Deny);
    assert_eq!(policy.decide(&destination, true, true, true), FormNavigation::Deny);

    policy.pending.set(true);
    assert_eq!(policy.decide(APP_URL, false, false, true), FormNavigation::Deny);
    policy.pending.set(false);
    policy.submitted.set(true);
    assert_eq!(
        policy.decide("https://other.example.test/redirect", true, false, true),
        FormNavigation::Deny
    );
    policy.submitted.set(false);
    policy.active.store(false, Ordering::Release);
    assert_eq!(policy.decide(&destination, true, false, true), FormNavigation::Deny);
}
