//! 验证敏感信息检测器对各类凭据的识别、阈值与脱敏。

use super::*;

#[test]
fn detector_allows_plain_text_and_flags_api_key_shape() {
    let detector = LeakDetector::default();
    assert!(matches!(detector.scan("plain operational message"), LeakResult::Clean));
    assert!(matches!(
        detector.scan("OPENAI_API_KEY=sk-abcdefghijklmnopqrstuvwxyz123456"),
        LeakResult::Detected { .. }
    ));
}

fn detected(content: &str) -> (Vec<String>, String) {
    match LeakDetector::new().scan(content) {
        LeakResult::Detected { patterns, redacted } => (patterns, redacted),
        LeakResult::Clean => panic!("expected leak detection for {content}"),
    }
}

#[test]
fn api_key_patterns_are_redacted_by_family() {
    for (content, expected) in [
        (concat!("stripe sk_live_", "1234567890abcdefghijklmnop"), "Stripe secret key"),
        ("anthropic sk-ant-1234567890abcdefghijklmnopqrstuvwxyz", "Anthropic API key"),
        ("google AIza1234567890abcdefghijklmnopqrstuvwxy", "Google API key"),
        ("github ghp_1234567890abcdefghijklmnopqrstuvwxyz123456", "GitHub token"),
        ("api_key=1234567890abcdefghijklmnop", "Generic API key"),
    ] {
        let (patterns, redacted) = detected(content);
        assert!(patterns.iter().any(|pattern| pattern == expected));
        assert!(redacted.contains("[REDACTED_API_KEY]"));
    }
}

#[test]
fn aws_credentials_are_redacted() {
    let (patterns, redacted) = detected(
        "AKIA1234567890ABCDEF aws_secret_access_key=1234567890abcdefghij1234567890abcdefghij",
    );

    assert!(patterns.contains(&"AWS Access Key ID".to_string()));
    assert!(patterns.contains(&"AWS Secret Access Key".to_string()));
    assert!(redacted.contains("[REDACTED_AWS_CREDENTIAL]"));
}

#[test]
fn generic_secret_patterns_obey_sensitivity_threshold() {
    let content = "password=supersecret token=abcdefghijklmnopqrstuv secret=abcdefghijklmnop";

    assert!(matches!(LeakDetector::with_sensitivity(0.5).scan(content), LeakResult::Clean));

    let result = LeakDetector::with_sensitivity(0.51).scan(content);
    match result {
        LeakResult::Detected { patterns, redacted } => {
            assert!(patterns.contains(&"Password in config".to_string()));
            assert!(patterns.contains(&"Token value".to_string()));
            assert!(patterns.contains(&"Secret value".to_string()));
            assert!(redacted.contains("[REDACTED_SECRET]"));
        }
        LeakResult::Clean => panic!("high sensitivity should detect generic secrets"),
    }
}

#[test]
fn private_key_jwt_and_database_urls_are_redacted() {
    let content = "\
-----BEGIN OPENSSH PRIVATE KEY-----
abc
-----END OPENSSH PRIVATE KEY-----
 bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.signature
 mysql://user:pass@localhost/db
 mongodb+srv://user:pass@example.mongodb.net/db
 redis://user:pass@localhost:6379";

    let (patterns, redacted) = detected(content);

    assert!(patterns.contains(&"OpenSSH private key".to_string()));
    assert!(patterns.contains(&"JWT token".to_string()));
    assert!(patterns.contains(&"MySQL connection URL".to_string()));
    assert!(patterns.contains(&"MongoDB connection URL".to_string()));
    assert!(patterns.contains(&"Redis connection URL".to_string()));
    assert!(redacted.contains("[REDACTED_PRIVATE_KEY]"));
    assert!(redacted.contains("[REDACTED_JWT]"));
    assert!(redacted.contains("[REDACTED_DATABASE_URL]"));
}

#[test]
fn sensitivity_is_clamped_to_supported_range() {
    let generic_secret = "secret=abcdefghijklmnop";
    assert!(matches!(
        LeakDetector::with_sensitivity(-10.0).scan(generic_secret),
        LeakResult::Clean
    ));
    assert!(matches!(
        LeakDetector::with_sensitivity(10.0).scan(generic_secret),
        LeakResult::Detected { .. }
    ));
}

#[test]
fn remaining_api_key_families_are_identified_and_redacted() {
    for (content, expected) in [
        (concat!("stripe pk_test_", "1234567890abcdefghijklmnop"), "Stripe publishable key"),
        (
            concat!("openai sk-", "12345678901234567890T3BlbkFJabcdefghijklmnopqrst"),
            "OpenAI API key",
        ),
        ("github github_pat_1234567890abcdefghijklmnopqrstuv", "GitHub PAT"),
    ] {
        let (patterns, redacted) = detected(content);
        assert!(patterns.iter().any(|pattern| pattern == expected));
        assert!(redacted.contains("[REDACTED_API_KEY]"));
    }
}

#[test]
fn pem_key_variants_and_postgresql_urls_are_redacted() {
    for (content, expected) in [
        ("-----BEGIN RSA PRIVATE KEY-----\nkey\n-----END RSA PRIVATE KEY-----", "RSA private key"),
        ("-----BEGIN EC PRIVATE KEY-----\nkey\n-----END EC PRIVATE KEY-----", "EC private key"),
        ("-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----", "Private key"),
        ("postgresql://user:pass@localhost/database", "PostgreSQL connection URL"),
    ] {
        let (patterns, redacted) = detected(content);
        assert!(patterns.iter().any(|pattern| pattern == expected));
        assert!(redacted.contains("[REDACTED_"));
    }
}
