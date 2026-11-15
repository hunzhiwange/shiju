use crate::domain_matcher::DomainMatcher;

// 测试精确域名匹配功能
#[test]
fn exact_match_works() {
    let matcher =
        DomainMatcher::new(&["accounts.google.com".to_string()], &[] as &[String]).unwrap();
    assert!(matcher.is_gated("accounts.google.com"));
    assert!(matcher.is_gated("https://accounts.google.com/login"));
    assert!(!matcher.is_gated("mail.google.com"));
}

// 测试通配符域名匹配功能
#[test]
fn wildcard_match_works() {
    let matcher = DomainMatcher::new(&["*.chase.com".to_string()], &[] as &[String]).unwrap();
    assert!(matcher.is_gated("www.chase.com"));
    assert!(matcher.is_gated("secure.chase.com"));
    assert!(!matcher.is_gated("chase.com"));
}

// 测试类别预设的扩展和匹配功能
#[test]
fn category_preset_expands_and_matches() {
    let matcher = DomainMatcher::new(&[] as &[String], &["banking".to_string()]).unwrap();
    assert!(matcher.is_gated("login.paypal.com"));
    assert!(matcher.is_gated("api.coinbase.com"));
    assert!(!matcher.is_gated("developer.mozilla.org"));
}

// 测试不匹配的域名返回 false
#[test]
fn non_matching_domain_returns_false() {
    let matcher =
        DomainMatcher::new(&["accounts.google.com".to_string()], &[] as &[String]).unwrap();
    assert!(!matcher.is_gated("example.com"));
}

// 测试畸形的域名模式会被拒绝
#[test]
fn malformed_domain_pattern_is_rejected() {
    let err = DomainMatcher::new(&["bad domain.com".to_string()], &[] as &[String])
        .expect_err("expected invalid pattern");
    assert!(err.to_string().contains("invalid characters"));
}

// 测试未知的类别会被拒绝
#[test]
fn unknown_category_is_rejected() {
    let err = DomainMatcher::new(&[] as &[String], &["unknown".to_string()])
        .expect_err("expected unknown category rejection");
    assert!(err.to_string().contains("Unknown OTP domain category"));
}
