//! Telegram 频道配对与绑定功能测试模块
//!
//! 本模块包含对 TelegramChannel 配对码激活状态和绑定码提取逻辑的单元测试。
//! 主要测试场景包括：
//! - 配对码在不同允许列表状态下的激活行为
//! - `/bind` 命令的解析与验证
//!
//! 相关模块：[`super`]（父模块，包含 TelegramChannel 实现）

use super::*;

/// 测试：空允许列表时配对码应处于激活状态
///
/// # 场景说明
/// 当 TelegramChannel 的允许列表（allowlist）为空时，表示未预设任何可信用户，
/// 此时需要通过配对码机制来绑定新用户，因此配对码应处于激活状态。
///
/// # 预期行为
/// - 使用空的允许列表创建 TelegramChannel
/// - `pairing_code_active()` 应返回 `true`
#[test]
fn telegram_pairing_enabled_with_empty_allowlist() {
    let ch = TelegramChannel::new("t".into(), vec![], false);
    assert!(ch.pairing_code_active());
}

/// 测试：非空允许列表时配对码应处于禁用状态
///
/// # 场景说明
/// 当 TelegramChannel 的允许列表包含预设用户时，表示已有可信用户配置，
/// 此时无需通过配对码来绑定用户，因此配对码应处于禁用状态。
///
/// # 预期行为
/// - 使用包含 "alice" 的允许列表创建 TelegramChannel
/// - `pairing_code_active()` 应返回 `false`
#[test]
fn telegram_pairing_disabled_with_nonempty_allowlist() {
    let ch = TelegramChannel::new("t".into(), vec!["alice".into()], false);
    assert!(!ch.pairing_code_active());
}

/// 测试：从纯命令格式中提取绑定码
///
/// # 场景说明
/// 测试从标准的 `/bind` 命令（不含机器人提及）中正确提取绑定码。
///
/// # 测试用例
/// - 输入：`"/bind 123456"`
/// - 预期输出：`Some("123456")`
#[test]
fn telegram_extract_bind_code_plain_command() {
    assert_eq!(TelegramChannel::extract_bind_code("/bind 123456"), Some("123456"));
}

/// 测试：从带机器人提及的命令格式中提取绑定码
///
/// # 场景说明
/// 测试从包含机器人用户名提及的 `/bind` 命令中正确提取绑定码。
/// 这种格式在群组中常见，用于明确指定目标机器人。
///
/// # 测试用例
/// - 输入：`"/bind@vibewindow_bot 654321"`
/// - 预期输出：`Some("654321")`
#[test]
fn telegram_extract_bind_code_supports_bot_mention() {
    assert_eq!(TelegramChannel::extract_bind_code("/bind@vibewindow_bot 654321"), Some("654321"));
}

/// 测试：拒绝无效的绑定码格式
///
/// # 场景说明
/// 测试对无效或不完整的 `/bind` 命令格式的拒绝处理。
/// 确保只有包含有效绑定码的命令才能被解析。
///
/// # 测试用例
/// - `/bind`（无绑定码参数）应返回 `None`
/// - `/start`（非绑定命令）应返回 `None`
#[test]
fn telegram_extract_bind_code_rejects_invalid_forms() {
    assert_eq!(TelegramChannel::extract_bind_code("/bind"), None);
    assert_eq!(TelegramChannel::extract_bind_code("/start"), None);
}
