//! 请求凭据日志预览与摘要的纯函数测试。

use super::{bearer_fingerprint, redact_bearer_for_log};

#[test]
fn redact_bearer_handles_empty_short_and_long_values() {
    assert_eq!(redact_bearer_for_log(""), "*");
    assert_eq!(redact_bearer_for_log("a"), "*");
    assert_eq!(redact_bearer_for_log("abcd"), "****");
    assert_eq!(redact_bearer_for_log("abcde"), "ab***de");
    assert_eq!(redact_bearer_for_log("token-value"), "to***ue");
}

#[test]
// 令牌预览按字符而非字节截断，避免破坏 UTF-8 边界。
fn redact_bearer_counts_unicode_characters_without_splitting_them() {
    assert_eq!(redact_bearer_for_log("密钥"), "**");
    assert_eq!(redact_bearer_for_log("甲乙丙丁戊"), "甲乙***丁戊");
}

#[test]
fn bearer_fingerprint_uses_first_twelve_hex_chars() {
    assert_eq!(bearer_fingerprint("secret"), "2bb80d537b1d");
}

#[test]
fn bearer_fingerprint_is_defined_for_an_empty_secret() {
    assert_eq!(bearer_fingerprint(""), "e3b0c44298fc");
}
