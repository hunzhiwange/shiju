//! Telegram 用户白名单功能测试模块
//!
//! 本模块提供针对 TelegramChannel 用户白名单验证功能的单元测试。
//! 主要测试场景包括：
//! - 通配符允许（允许所有用户）
//! - 特定用户白名单验证
//! - 用户名格式处理（带 @ 前缀）
//! - 空白名单拒绝策略
//! - 精确匹配验证（非子串匹配）
//! - 大小写敏感性验证
//! - 数字 ID 身份验证
//! - 多标识身份匹配逻辑

use super::*;

/// 测试通配符 "*" 允许所有用户访问
///
/// 验证当白名单包含 "*" 通配符时，任何用户都应该被允许访问。
/// 这是 Telegram 频道最宽松的访问控制策略。
#[test]
fn telegram_user_allowed_wildcard() {
    let ch = TelegramChannel::new("t".into(), vec!["*".into()], false);
    assert!(ch.is_any_user_allowed(["anyone"].into_iter()));
}

/// 测试特定用户白名单验证
///
/// 验证只有白名单中的用户才能访问，未在白名单中的用户将被拒绝。
/// 使用多个白名单用户名进行测试，确保精确匹配逻辑正确。
#[test]
fn telegram_user_allowed_specific() {
    let ch = TelegramChannel::new("t".into(), vec!["alice".into(), "bob".into()], false);
    assert!(ch.is_any_user_allowed(["alice"].into_iter()));
    assert!(!ch.is_any_user_allowed(["eve"].into_iter()));
}

/// 测试配置中带 @ 前缀的用户名处理
///
/// 在 Telegram 中，用户名通常以 @ 开头。
/// 验证白名单配置中包含 @alice 时，输入 alice 应该能够匹配通过。
/// 这确保了用户名格式的灵活性和容错性。
#[test]
fn telegram_user_allowed_with_at_prefix_in_config() {
    let ch = TelegramChannel::new("t".into(), vec!["@alice".into()], false);
    assert!(ch.is_any_user_allowed(["alice"].into_iter()));
}

/// 测试空白名单拒绝所有用户
///
/// 安全默认策略：当白名单为空时，应该拒绝所有用户的访问请求。
/// 这是重要的安全边界测试，确保在没有明确允许的情况下不开放访问。
#[test]
fn telegram_user_denied_empty() {
    let ch = TelegramChannel::new("t".into(), vec![], false);
    assert!(!ch.is_any_user_allowed(["anyone"].into_iter()));
}

/// 测试精确匹配验证（非子串匹配）
///
/// 验证用户白名单必须是精确匹配，而不是子串匹配。
/// 例如，白名单中的 "alice" 不应该匹配 "alice_bot"、"alic" 或 "malice"。
/// 这防止了因用户名相似性导致的安全漏洞。
#[test]
fn telegram_user_exact_match_not_substring() {
    let ch = TelegramChannel::new("t".into(), vec!["alice".into()], false);
    assert!(!ch.is_any_user_allowed(["alice_bot"].into_iter()));
    assert!(!ch.is_any_user_allowed(["alic"].into_iter()));
    assert!(!ch.is_any_user_allowed(["malice"].into_iter()));
}

/// 测试空字符串用户名被拒绝
///
/// 验证空字符串不应该匹配任何白名单用户。
/// 这是一种边界情况测试，防止因空输入导致的安全问题。
#[test]
fn telegram_user_empty_string_denied() {
    let ch = TelegramChannel::new("t".into(), vec!["alice".into()], false);
    assert!(!ch.is_any_user_allowed([""].into_iter()));
}

/// 测试用户名大小写敏感性
///
/// 验证用户白名单匹配是大小写敏感的。
/// 白名单中的 "Alice" 只匹配 "Alice"，不匹配 "alice" 或 "ALICE"。
/// 这确保了用户名验证的严格性和一致性。
#[test]
fn telegram_user_case_sensitive() {
    let ch = TelegramChannel::new("t".into(), vec!["Alice".into()], false);
    assert!(ch.is_any_user_allowed(["Alice"].into_iter()));
    assert!(!ch.is_any_user_allowed(["alice"].into_iter()));
    assert!(!ch.is_any_user_allowed(["ALICE"].into_iter()));
}

/// 测试通配符与特定用户混合配置
///
/// 验证当白名单同时包含通配符 "*" 和特定用户名时，
/// 所有用户（包括白名单中的特定用户）都应该被允许访问。
/// 这确保了通配符的优先级和正确行为。
#[test]
fn telegram_wildcard_with_specific_users() {
    let ch = TelegramChannel::new("t".into(), vec!["alice".into(), "*".into()], false);
    assert!(ch.is_any_user_allowed(["alice"].into_iter()));
    assert!(ch.is_any_user_allowed(["bob"].into_iter()));
    assert!(ch.is_any_user_allowed(["anyone"].into_iter()));
}

/// 测试通过数字 ID 标识允许用户
///
/// Telegram 用户可以使用数字 ID 进行标识。
/// 验证当用户提供多个身份标识时，只要其中任何一个匹配白名单，
/// 就应该允许该用户访问。
#[test]
fn telegram_user_allowed_by_numeric_id_identity() {
    let ch = TelegramChannel::new("t".into(), vec!["123456789".into()], false);
    // 用户提供两个身份标识："unknown" 和 "123456789"
    // 由于 "123456789" 在白名单中，应该允许访问
    assert!(ch.is_any_user_allowed(["unknown", "123456789"].into_iter()));
}

/// 测试当没有任何身份标识匹配白名单时拒绝访问
///
/// 验证当用户提供的所有身份标识都不在白名单中时，应该拒绝访问。
/// 这测试了多身份标识场景下的完整拒绝逻辑。
#[test]
fn telegram_user_denied_when_none_of_identities_match() {
    let ch = TelegramChannel::new("t".into(), vec!["alice".into(), "987654321".into()], false);
    // 用户提供的两个身份标识 "unknown" 和 "123456789" 都不在白名单中
    assert!(!ch.is_any_user_allowed(["unknown", "123456789"].into_iter()));
}
