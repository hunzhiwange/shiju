//! 测试代理循环通用辅助函数的输入规范化与边界行为。

use super::utils::{SENSITIVE_KEY_PATTERNS, TOOL_FILE_PATH_REGEX, scrub_credentials};

#[test]
fn scrub_credentials_redacts_sensitive_values_without_touching_plain_text() {
    let input = r#"token: "abcdef123456" api_key=xyz987654321 plain=value"#;
    let scrubbed = scrub_credentials(input);

    assert!(scrubbed.contains("abcd*[REDACTED]"));
    assert!(scrubbed.contains("xyz9*[REDACTED]"));
    assert!(scrubbed.contains("plain=value"));
    assert!(!scrubbed.contains("ef123456"));
}

#[test]
fn scrub_credentials_handles_supported_key_and_separator_variants() {
    let input = r#"
        "password": "supersecret"
        secret='abcdefghijk'
        USER-KEY=abcdEFGH1234
        bearer: bearerToken999
        credential = credVALUE42
    "#;

    let scrubbed = scrub_credentials(input);

    assert!(scrubbed.contains(r#""password": "supe*[REDACTED]""#));
    assert!(scrubbed.contains("secret=abcd*[REDACTED]"));
    assert!(scrubbed.contains("USER-KEY=abcd*[REDACTED]"));
    assert!(scrubbed.contains("bearer: bear*[REDACTED]"));
    assert!(scrubbed.contains("credential=cred*[REDACTED]"));
    assert!(!scrubbed.contains("supersecret"));
    assert!(!scrubbed.contains("EFGH1234"));
}

#[test]
fn scrub_credentials_redacts_double_quoted_equals_value_by_chars() {
    let input = r#"api_key="密钥凭证安全测试""#;
    let scrubbed = scrub_credentials(input);

    assert_eq!(scrubbed, r#"api_key="密钥凭证*[REDACTED]""#);
}

#[test]
fn scrub_credentials_redacts_colon_pairs_without_double_quotes() {
    let input = "secret: 'abcdefghijk' bearer: bearerToken999";
    let scrubbed = scrub_credentials(input);

    assert_eq!(scrubbed, "secret: abcd*[REDACTED] bearer: bear*[REDACTED]");
}

#[test]
fn scrub_credentials_ignores_short_or_unsupported_values() {
    let input = r#"token=short api_key=abc123!@# note="password: not a pair""#;

    let scrubbed = scrub_credentials(input);

    assert_eq!(scrubbed, input);
}

#[test]
fn sensitive_key_patterns_match_expected_names_case_insensitively() {
    for key in [
        "TOKEN",
        "api_key",
        "api-key",
        "Password",
        "client_secret",
        "USER_KEY",
        "user-key",
        "Bearer",
        "credential_id",
    ] {
        assert!(SENSITIVE_KEY_PATTERNS.is_match(key), "{key} should be sensitive");
    }

    assert!(!SENSITIVE_KEY_PATTERNS.is_match("ordinary_field"));
}

#[test]
fn tool_file_path_regex_extracts_file_path_and_path_attributes() {
    let output = r#"file_path: "/tmp/report.txt" path="/var/log/app.log""#;
    let paths = TOOL_FILE_PATH_REGEX
        .captures_iter(output)
        .map(|caps| caps[1].to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["/tmp/report.txt", "/var/log/app.log"]);
}

#[test]
fn tool_file_path_regex_is_case_insensitive_and_skips_unquoted_paths() {
    let output = r#"PATH="/tmp/upper.log" file_path=/tmp/unquoted.txt ignored="path=/tmp/nested""#;
    let paths = TOOL_FILE_PATH_REGEX
        .captures_iter(output)
        .map(|caps| caps[1].to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["/tmp/upper.log"]);
}
