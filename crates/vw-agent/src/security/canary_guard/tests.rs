//! CanaryGuard 安全防护机制测试模块
//!
//! 本模块提供针对 `CanaryGuard` 金丝雀防护机制的全面单元测试，
//! 验证令牌注入、泄漏检测和日志脱敏等核心安全功能的正确性。
//!
//! # 测试覆盖范围
//!
//! - 金丝雀令牌注入功能（启用/禁用状态）
//! - 令牌轮换机制
//! - 泄漏检测能力
//! - 敏感信息脱敏处理

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试禁用金丝雀防护时的行为
    ///
    /// 当 CanaryGuard 被配置为禁用状态时，应该：
    /// - 不修改原始提示词
    /// - 不返回任何令牌
    ///
    /// # 验证点
    /// - 返回的提示词与输入完全一致
    /// - 返回的令牌为 None
    #[test]
    fn inject_turn_token_disabled_returns_prompt_without_token() {
        let guard = CanaryGuard::new(false);
        let (prompt, token) = guard.inject_turn_token("system prompt");

        assert_eq!(prompt, "system prompt");
        assert!(token.is_none());
    }

    #[test]
    fn disabled_guard_strips_stale_canary_block() {
        let guard = CanaryGuard::new(true);
        let (prompt_with_canary, token) = guard.inject_turn_token("system prompt");
        assert!(token.is_some());

        let disabled = CanaryGuard::new(false);
        let (clean_prompt, clean_token) = disabled.inject_turn_token(&prompt_with_canary);

        assert_eq!(clean_prompt, "system prompt\n");
        assert!(clean_token.is_none());
        assert!(!clean_prompt.contains(CANARY_START_MARKER));
    }

    #[test]
    fn enabled_injection_adds_newline_when_prompt_lacks_one() {
        let guard = CanaryGuard::new(true);
        let (prompt, token) = guard.inject_turn_token("base");

        let token = token.expect("enabled guard should return token");
        assert!(prompt.starts_with("base\n"));
        assert!(prompt.contains(&token));
        assert!(token.starts_with("ZCSEC-"));
        assert_eq!(token.len(), "ZCSEC-".len() + 12);
    }

    /// 测试金丝雀令牌的轮换机制
    ///
    /// 当连续多次调用令牌注入时，应该：
    /// - 每次生成不同的唯一令牌
    /// - 自动替换已存在的金丝雀块
    /// - 保持提示词中只有一个金丝雀标记块
    ///
    /// # 验证点
    /// - 每次调用都返回有效的令牌（Some）
    /// - 连续两次调用返回的令牌不同
    /// - 提示词中始终只包含一对金丝雀标记（开始和结束标记各一个）
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

    /// 测试金丝雀令牌泄漏检测和日志脱敏功能
    ///
    /// 模拟攻击者尝试泄露金丝雀令牌的场景，验证：
    /// - 能够准确检测响应中是否包含令牌
    /// - 能够在日志中正确脱敏令牌内容
    ///
    /// # 验证点
    /// - `response_contains_canary` 能够正确识别泄漏
    /// - `redact_token_from_text` 能够将令牌替换为安全占位符
    /// - 脱敏后的文本不包含原始令牌
    /// - 脱敏后的文本包含 `[REDACTED_CANARY]` 占位符
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

    #[test]
    fn blank_or_disabled_tokens_do_not_match_or_redact() {
        let enabled = CanaryGuard::new(true);
        assert!(!enabled.response_contains_canary("token", None));
        assert!(!enabled.response_contains_canary("token", Some("  ")));
        assert_eq!(enabled.redact_token_from_text("token", Some("  ")), "token");

        let disabled = CanaryGuard::new(false);
        assert!(!disabled.response_contains_canary("ZCSEC-ABC", Some("ZCSEC-ABC")));
        assert_eq!(
            disabled.redact_token_from_text("ZCSEC-ABC", Some("ZCSEC-ABC")),
            "[REDACTED_CANARY]"
        );
    }

    #[test]
    fn strip_canary_block_preserves_unterminated_marker_text() {
        let prompt = format!("base\n{CANARY_START_MARKER}\nunterminated");

        assert_eq!(strip_canary_block(&prompt), prompt);
    }

    #[test]
    fn strip_canary_block_collapses_newlines_around_removed_block() {
        let prompt = format!("before\n{CANARY_START_MARKER}\nsecret\n{CANARY_END_MARKER}\nafter");

        assert_eq!(strip_canary_block(&prompt), "before\nafter");
    }
}
