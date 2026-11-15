//! # 通道消息超时预算测试模块
//!
//! 本模块提供 `channels` 模块中消息超时相关函数的单元测试，
//! 用于验证超时时间计算和预算分配的正确性。
//!
//! ## 测试覆盖
//!
//! - **最小超时限制**：验证超时时间不低于安全阈值
//! - **预算线性扩展**：验证超时预算随工具迭代次数正确扩展
//! - **边界安全保护**：验证零值和超大值的防护机制
//!
//! ## 被测函数
//!
//! - `effective_channel_message_timeout_secs`: 计算有效的消息超时时间
//! - `channel_message_timeout_budget_secs`: 计算基于迭代的超时预算
//!
//! ## 相关常量
//!
//! - `MIN_CHANNEL_MESSAGE_TIMEOUT_SECS`: 最小超时时间下限（30秒）
//! - `CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP`: 超时缩放上限系数（4）

use super::*;

/// 测试：有效消息超时时间应用最小值限制
///
/// 验证 `effective_channel_message_timeout_secs` 函数能够正确处理
/// 低于最小阈值的配置值，确保超时时间始终保持在安全范围内。
///
/// # 测试场景
///
/// 1. **零值输入**：配置值为 0 时，应返回最小阈值（30秒）
/// 2. **低于阈值**：配置值为 15 时，应返回最小阈值（30秒）
/// 3. **正常值**：配置值为 300 时，应直接使用配置值（300秒）
///
/// # 验证要点
///
/// - 任何低于 30 秒的配置都应被提升到最小阈值
/// - 等于或高于最小阈值的配置应保持不变
/// - 防止因配置过短导致消息处理失败
#[test]
fn effective_channel_message_timeout_secs_clamps_to_minimum() {
    // 零值测试：配置为 0 秒，期望提升到最小阈值
    assert_eq!(effective_channel_message_timeout_secs(0), MIN_CHANNEL_MESSAGE_TIMEOUT_SECS);
    // 低值测试：配置为 15 秒（低于阈值），期望提升到最小阈值
    assert_eq!(effective_channel_message_timeout_secs(15), MIN_CHANNEL_MESSAGE_TIMEOUT_SECS);
    // 正常值测试：配置为 300 秒（高于阈值），期望保持原值
    assert_eq!(effective_channel_message_timeout_secs(300), 300);
}

/// 测试：通道消息超时预算随工具迭代次数线性扩展
///
/// 验证 `channel_message_timeout_budget_secs` 函数能够正确计算
/// 基于工具迭代次数的超时预算，确保有足够时间完成复杂任务。
///
/// # 测试场景
///
/// 1. **单次迭代**：基础超时 300 秒 × 1 次迭代 = 300 秒预算
/// 2. **两次迭代**：基础超时 300 秒 × 2 次迭代 = 600 秒预算
/// 3. **三次迭代**：基础超时 300 秒 × 3 次迭代 = 900 秒预算
///
/// # 计算逻辑
///
/// 总超时预算 = 消息超时时间 × min(迭代次数, 缩放上限)
///
/// # 验证要点
///
/// - 超时预算应与迭代次数成正比
/// - 每增加一次迭代，预算增加一个基础超时时间
/// - 确保多轮工具执行有充足的完成时间
#[test]
fn channel_message_timeout_budget_scales_with_tool_iterations() {
    // 1 次迭代：预算 = 300 × 1 = 300 秒
    assert_eq!(channel_message_timeout_budget_secs(300, 1), 300);
    // 2 次迭代：预算 = 300 × 2 = 600 秒
    assert_eq!(channel_message_timeout_budget_secs(300, 2), 600);
    // 3 次迭代：预算 = 300 × 3 = 900 秒
    assert_eq!(channel_message_timeout_budget_secs(300, 3), 900);
}

/// 测试：通道消息超时预算使用安全默认值和上限
///
/// 验证 `channel_message_timeout_budget_secs` 函数对边界情况的处理：
/// - 零值迭代次数的防护
/// - 超大迭代次数的上限限制
///
/// # 测试场景
///
/// 1. **零次迭代**：配置为 0 次时，应使用最小值 1 次计算
///    - 预算 = 300 × 1 = 300 秒
/// 2. **超大迭代**：配置为 10 次时，应受缩放上限（4）限制
///    - 预算 = 300 × 4（上限）= 1200 秒
///
/// # 防护机制
///
/// - **最小迭代保护**：迭代次数至少为 1，避免零值或负值
/// - **上限保护**：缩放系数不超过 `CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP`（4）
///
/// # 验证要点
///
/// - 零值迭代应自动提升到 1
/// - 超过上限的迭代次数应被截断到上限值
/// - 防止过大配置导致无界等待或整数溢出
#[test]
fn channel_message_timeout_budget_uses_safe_defaults_and_cap() {
    // 零次迭代测试：应提升到 1 次计算
    // 期望预算 = 300 × 1 = 300 秒
    assert_eq!(channel_message_timeout_budget_secs(300, 0), 300);
    // 超大迭代测试：10 次超过上限 4，应截断到上限
    // 期望预算 = 300 × 4（上限）= 1200 秒
    assert_eq!(
        channel_message_timeout_budget_secs(300, 10),
        300 * CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP
    );
}
