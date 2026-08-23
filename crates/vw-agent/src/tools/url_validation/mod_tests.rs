//! 验证网址规范化、主机提取、私网识别及域名策略边界。
//! 允许与阻止列表、协议限制及缺失主机必须按拒绝优先顺序处理。

use super::*;

fn policy<'a>(allowed: &'a [String], blocked: &'a [String]) -> DomainPolicy<'a> {
    DomainPolicy {
        allowed_domains: allowed,
        blocked_domains: blocked,
        allowed_field_name: "allowed",
        blocked_field_name: Some("blocked"),
        empty_allowed_message: "empty allowed",
        scheme_policy: UrlSchemePolicy::HttpOrHttps,
        ipv6_error_context: "tests",
    }
}

#[test]
fn normalization_host_extraction_matching_and_private_detection() {
    assert_eq!(
        normalize_domain(" HTTPS://API.EXAMPLE.COM:443/path "),
        Some("api.example.com".into())
    );
    assert_eq!(normalize_domain("bad host"), None);
    assert_eq!(
        normalize_allowed_domains(vec!["B.test".into(), "b.test".into(), "".into()]),
        vec!["b.test".to_string()]
    );

    assert_eq!(
        extract_host("https://Example.COM.:443/path", UrlSchemePolicy::HttpsOnly, "ctx").unwrap(),
        "example.com"
    );
    assert!(extract_host("http://example.com", UrlSchemePolicy::HttpsOnly, "ctx").is_err());
    assert!(extract_host("https://u@example.com", UrlSchemePolicy::HttpsOnly, "ctx").is_err());
    assert!(extract_host("https://[::1]", UrlSchemePolicy::HttpsOnly, "ctx").is_err());

    let allow = vec!["example.com".to_string(), "*.cdn.test".to_string()];
    assert!(host_matches_allowlist("api.example.com", &allow));
    assert!(host_matches_allowlist("cdn.test", &allow));
    assert!(!host_matches_allowlist("other.test", &allow));

    for host in [
        "localhost",
        "a.localhost",
        "printer.local",
        "127.0.0.1",
        "10.0.0.1",
        "100.64.0.1",
        "198.51.100.1",
        "[::1]",
        "2001:db8::1",
    ] {
        assert!(is_private_or_local_host(host));
    }
    assert!(!is_private_or_local_host("example.com"));
}

#[test]
fn validate_url_enforces_policy_edges() {
    let allowed = vec!["example.com".to_string()];
    let blocked = vec!["blocked.example.com".to_string()];
    let domain_policy = policy(&allowed, &blocked);

    assert_eq!(
        validate_url(" https://api.example.com/path ", &domain_policy).unwrap(),
        "https://api.example.com/path"
    );
    assert!(validate_url("", &domain_policy).unwrap_err().to_string().contains("empty"));
    assert!(
        validate_url("https://example.com/a b", &domain_policy)
            .unwrap_err()
            .to_string()
            .contains("whitespace")
    );
    assert!(
        validate_url("https://blocked.example.com", &domain_policy)
            .unwrap_err()
            .to_string()
            .contains("blocked")
    );
    assert!(
        validate_url("https://other.com", &domain_policy)
            .unwrap_err()
            .to_string()
            .contains("not in allowed")
    );
    assert!(
        validate_url("http://127.0.0.1", &domain_policy)
            .unwrap_err()
            .to_string()
            .contains("local/private")
    );

    let empty: Vec<String> = vec![];
    assert_eq!(
        validate_url("https://example.com", &policy(&empty, &[])).unwrap_err().to_string(),
        "empty allowed"
    );
}

#[test]
fn extract_host_rejects_port_without_hostname() {
    let error = extract_host("https://:443/path", UrlSchemePolicy::HttpsOnly, "tests")
        .expect_err("an authority containing only a port must be rejected");

    assert_eq!(error.to_string(), "URL must include a valid host");
}
