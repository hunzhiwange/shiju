use super::*;

#[test]
fn domain_helpers_normalize_and_match_allowlist() {
    let allowed = normalize_domains(vec![
        " Example.COM ".to_string(),
        "*.trusted.test".to_string(),
        " ".to_string(),
    ]);
    assert_eq!(allowed, vec!["example.com", "*.trusted.test"]);
    assert!(host_matches_allowlist("www.example.com", &allowed));
    assert!(host_matches_allowlist("api.trusted.test", &allowed));
    assert!(host_matches_allowlist("trusted.test", &allowed));
    assert!(host_matches_allowlist("anything.invalid", &["*".to_string()]));
    assert!(!host_matches_allowlist("example.net", &allowed));
}

#[test]
fn host_extraction_and_private_detection_are_conservative() {
    assert_eq!(extract_host("https://Example.COM:443/path").unwrap(), "example.com");
    assert_eq!(extract_host("http://[::1]:8080/").unwrap(), "[::1]");
    assert_eq!(extract_host("example.com:443/path").unwrap(), "example.com");
    assert!(extract_host("https:///missing-host").is_err());
    assert!(is_private_host("localhost"));
    assert!(is_private_host("api.localhost"));
    assert!(is_private_host("printer.local"));
    assert!(is_private_host("192.168.1.1"));
    assert!(is_private_host("10.1.2.3"));
    assert!(is_private_host("172.16.0.1"));
    assert!(is_private_host("169.254.1.1"));
    assert!(is_private_host("100.64.0.1"));
    assert!(is_private_host("192.0.2.1"));
    assert!(is_private_host("198.18.0.1"));
    assert!(is_private_host("198.51.100.1"));
    assert!(is_private_host("203.0.113.1"));
    assert!(is_private_host("240.0.0.1"));
    assert!(is_private_host("[::1]"));
    assert!(is_private_host("fc00::1"));
    assert!(is_private_host("fe80::1"));
    assert!(is_private_host("::ffff:192.168.1.1"));
    assert!(!is_private_host("example.com"));
    assert!(!is_private_host("8.8.8.8"));
}

#[test]
fn recoverable_error_detection_matches_known_transient_shapes() {
    for message in [
        "invalid session id",
        "no such window",
        "session not created",
        "connection reset",
        "broken pipe",
        "WebDriver request timed out",
        "webdriver timeout",
    ] {
        assert!(is_recoverable_rust_native_error(&anyhow::anyhow!(message)), "{message}");
    }

    assert!(!is_recoverable_rust_native_error(&anyhow::anyhow!("element not found")));
}

#[test]
fn endpoint_reachable_handles_missing_ports_and_unreachable_hosts() {
    let file_url = reqwest::Url::parse("file:///tmp/out").unwrap();
    assert!(!endpoint_reachable(&file_url, std::time::Duration::from_millis(1)));

    let http_url = reqwest::Url::parse("http://127.0.0.1:9").unwrap();
    assert!(!endpoint_reachable(&http_url, std::time::Duration::from_millis(1)));

    let unknown_scheme = reqwest::Url::parse("custom://example.com/resource").unwrap();
    assert!(!endpoint_reachable(&unknown_scheme, std::time::Duration::from_millis(1)));
}

#[test]
fn private_host_detection_covers_special_and_public_address_boundaries() {
    for host in [
        "0.0.0.0",
        "127.255.255.255",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "ff02::1",
        "::ffff:100.64.0.1",
    ] {
        assert!(is_private_host(host), "{host} should be treated as non-global");
    }

    for host in ["1.1.1.1", "2001:4860:4860::8888", "::ffff:8.8.8.8"] {
        assert!(!is_private_host(host), "{host} should remain globally routable");
    }
}
