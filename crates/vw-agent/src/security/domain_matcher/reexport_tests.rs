use crate::security::DomainMatcher;

#[test]
fn domain_matcher_reexport_preserves_identity_and_matching() {
    let matcher = DomainMatcher::new(&["accounts.google.com".to_string()], &[]).unwrap();
    let matcher: &::domain_matcher::domain_matcher::DomainMatcher = &matcher;

    assert!(matcher.is_gated("https://accounts.google.com/login"));
    assert!(!matcher.is_gated("mail.google.com"));
}
