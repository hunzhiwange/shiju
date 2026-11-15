//! Discord 表情反应处理模块
//!
//! 本模块提供 Discord 消息表情反应的相关功能，包括：
//! - 随机选择确认表情
//! - 表情编码转换
//! - 生成表情反应 API URL
//!
//! 主要用于向 Discord 消息添加表情反应，作为对用户消息的确认或反馈。

use std::fmt::Write as _;

/// Discord 确认表情列表
///
/// 包含一系列用于消息确认的 Unicode 表情符号。
/// 这些表情会在需要确认收到消息时随机选择使用。
const DISCORD_ACK_REACTIONS: &[&str] = &["⚡️", "🦀", "🙌", "💪", "👌", "👀", "👣"];
const DISCORD_API_BASE_URL: &str = "https://discord.com/api/v10";

/// 生成均匀分布的随机索引
///
/// 使用拒绝采样算法生成一个在 [0, len) 范围内的均匀随机索引。
/// 避免了简单取模方法在小范围上可能产生的模偏差。
///
/// # 参数
///
/// * `len` - 索引的上界（不包含），必须大于 0
///
/// # 返回
///
/// 返回一个在 [0, len) 范围内的随机索引值
///
/// # 示例
///
/// ```
/// let index = pick_uniform_index(7);
/// assert!(index < 7);
/// ```
///
/// # 实现说明
///
/// 使用拒绝采样避免模偏差：
/// - 计算拒绝阈值：`reject_threshold = (u64::MAX / len) * len`
/// - 如果随机值超过阈值，则拒绝并重新采样
/// - 否则返回 `value % len`
#[allow(clippy::cast_possible_truncation)]
fn pick_uniform_index(len: usize) -> usize {
    debug_assert!(len > 0);
    let upper = len as u64;
    // 计算拒绝阈值，用于消除模偏差
    let reject_threshold = (u64::MAX / upper) * upper;

    loop {
        // 生成 64 位随机数
        let value = rand::random::<u64>();
        // 如果随机值在均匀分布范围内，则接受
        if value < reject_threshold {
            return (value % upper) as usize;
        }
        // 否则拒绝并重新采样
    }
}

/// 随机选择一个 Discord 确认表情
///
/// 从预定义的确认表情列表中均匀随机选择一个表情。
///
/// # 返回
///
/// 返回一个静态生命周期的字符串切片，包含选中的表情符号
///
/// # 示例
///
/// ```
/// let emoji = random_discord_ack_reaction();
/// // emoji 可能是 "⚡️", "🦀", "🙌" 等之一
/// ```
pub(super) fn random_discord_ack_reaction() -> &'static str {
    DISCORD_ACK_REACTIONS[pick_uniform_index(DISCORD_ACK_REACTIONS.len())]
}

/// 将 Unicode 表情编码为 Discord API 可用的 URL 格式
///
/// Discord 的表情反应端点要求表情符号在 URL 路径中进行百分号编码。
/// 对于自定义服务器表情（格式为 `name:id`），则直接传递不进行编码。
///
/// # 参数
///
/// * `emoji` - 要编码的表情符号字符串，可以是 Unicode 表情或自定义表情
///
/// # 返回
///
/// 返回编码后的字符串：
/// - 对于 Unicode 表情：返回百分号编码的字符串（例如 "⚡️" -> "%E2%9A%A1%EF%B8%8F"）
/// - 对于自定义表情：原样返回
///
/// # 示例
///
/// ```
/// let encoded = encode_emoji_for_discord("⚡️");
/// assert_eq!(encoded, "%E2%9A%A1%EF%B8%8F");
///
/// let custom = encode_emoji_for_discord("custom:123456");
/// assert_eq!(custom, "custom:123456");
/// ```
pub(super) fn encode_emoji_for_discord(emoji: &str) -> String {
    // 自定义表情包含冒号，直接返回不编码
    if emoji.contains(':') {
        return emoji.to_string();
    }

    // 对每个字节进行百分号编码
    let mut encoded = String::new();
    for byte in emoji.as_bytes() {
        write!(encoded, "%{byte:02X}").ok();
    }
    encoded
}

/// 构建 Discord 表情反应 API URL
///
/// 生成用于向 Discord 消息添加表情反应的完整 API URL。
///
/// # 参数
///
/// * `channel_id` - Discord 频道 ID
/// * `message_id` - Discord 消息 ID，可能带有 "discord_" 前缀
/// * `emoji` - 要添加的表情符号
///
/// # 返回
///
/// 返回完整的 Discord API URL，格式为：
/// `https://discord.com/api/v10/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me`
///
/// # 示例
///
/// ```
/// let url = discord_reaction_url("123456", "789012", "👍");
/// // 返回类似：https://discord.com/api/v10/channels/123456/messages/789012/reactions/%F0%9F%91%8D/@me
/// ```
pub(super) fn discord_reaction_url(channel_id: &str, message_id: &str, emoji: &str) -> String {
    discord_reaction_url_with_base(DISCORD_API_BASE_URL, channel_id, message_id, emoji)
}

pub(super) fn discord_reaction_url_with_base(
    api_base_url: &str,
    channel_id: &str,
    message_id: &str,
    emoji: &str,
) -> String {
    // 移除消息 ID 的 "discord_" 前缀（如果存在）
    let raw_id = message_id.strip_prefix("discord_").unwrap_or(message_id);
    // 编码表情符号
    let encoded_emoji = encode_emoji_for_discord(emoji);
    let api_base_url = api_base_url.trim_end_matches('/');
    // 构建完整的 API URL
    format!("{api_base_url}/channels/{channel_id}/messages/{raw_id}/reactions/{encoded_emoji}/@me")
}

/// 获取确认表情列表（仅用于测试）
///
/// 返回预定义的确认表情列表的引用，用于测试目的。
///
/// # 返回
///
/// 返回确认表情列表的静态切片
///
/// # 注意
///
/// 此函数仅在测试配置下可用
#[cfg(test)]
pub(super) fn discord_ack_reactions() -> &'static [&'static str] {
    DISCORD_ACK_REACTIONS
}

#[cfg(test)]
#[path = "reactions_tests.rs"]
mod reactions_tests;
