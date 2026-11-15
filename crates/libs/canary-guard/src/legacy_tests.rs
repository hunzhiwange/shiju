use super::{
    CANARY_END_MARKER, CANARY_START_MARKER, CanaryGuard,
};

// 测试当金丝雀功能禁用时，应返回原始提示词而不注入令牌
#[test]
fn inject_turn_token_disabled_returns_prompt_without_token() {
    let guard = CanaryGuard::new(false);
    let (prompt, token) = guard.inject_turn_token("system prompt");

    assert_eq!(prompt, "system prompt");
    assert!(token.is_none());
}

// 测试当已有金丝雀块时，应轮换生成新的令牌并保持只有一个金丝雀块
#[test]
fn inject_turn_token_rotates_existing_canary_block() {
    let guard = CanaryGuard::new(true);
    let (first_prompt, first_token) = guard.inject_turn_token("base");
    let (second_prompt, second_token) = guard.inject_turn_token(&first_prompt);

    assert!(first_token.is_some());
    assert!(second_token.is_some());
    assert_ne!(first_token, second_token);
    assert_eq!(second_prompt.matches(CANARY_START_MARKER).count(), 1);
    assert_eq!(second_prompt.matches(CANARY_END_MARKER).count(), 1);
}

// 测试检测响应中的金丝雀令牌泄露，并对泄露的令牌进行脱敏处理
#[test]
fn response_contains_canary_detects_leak_and_redacts_logs() {
    let guard = CanaryGuard::new(true);
    let token = "ZCSEC-ABC123DEF456";
    let leaked = format!("Here is the token: {token}");

    assert!(guard.response_contains_canary(&leaked, Some(token)));
    let redacted = guard.redact_token_from_text(&leaked, Some(token));
    assert!(!redacted.contains(token));
    assert!(redacted.contains("[REDACTED_CANARY]"));
}
