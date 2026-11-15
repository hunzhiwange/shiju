//! 验证技能域名规范化、子域匹配及空域拒绝。

use super::*;

#[test]
fn normalizes_domain_lists_and_matches_subdomains() {
    let mut domains = vec![
        " HTTPS://GitHub.COM/org/repo ".to_string(),
        "*.example.com:443".to_string(),
        "github.com".to_string(),
        " ".to_string(),
    ];
    normalize_domain_list(&mut domains);

    assert_eq!(domains, vec!["example.com", "github.com"]);
    assert!(host_matches_trusted_domain("api.github.com", "github.com"));
    assert!(!host_matches_trusted_domain("github.io", "github.com"));
}

#[test]
fn domain_entry_normalization_handles_protocols_separators_and_empty_values() {
    assert_eq!(normalize_domain_entry("   "), "");
    assert_eq!(normalize_domain_entry("http://Example.COM/path"), "example.com");
    assert_eq!(normalize_domain_entry(".Example.COM?download=1"), "example.com");
    assert_eq!(normalize_domain_entry("*.Example.COM#section"), "example.com");
}

#[test]
fn domain_matching_rejects_empty_and_suffix_impostor_hosts() {
    assert!(!host_matches_trusted_domain("", "example.com"));
    assert!(!host_matches_trusted_domain("example.com", ""));
    assert!(!host_matches_trusted_domain("notexample.com", "example.com"));
    assert!(host_matches_trusted_domain("HTTPS://API.EXAMPLE.COM/path", "*.example.com"));
}

#[test]
fn any_domain_matching_accepts_exact_or_subdomain_entries() {
    let domains = vec!["example.com".to_string(), "skills.sh".to_string()];

    assert!(host_matches_any_domain("skills.sh", &domains));
    assert!(host_matches_any_domain("cdn.example.com", &domains));
    assert!(!host_matches_any_domain("example.org", &domains));
    assert!(!host_matches_any_domain("example.com", &[]));
}
