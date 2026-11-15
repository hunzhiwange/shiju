use super::super::reactions as discord_reactions;
use super::*;

/// 测试 Unicode 表情符号的百分比编码
/// 眼睛表情符号 (U+1F440) 应该被正确编码
#[test]
fn encode_emoji_unicode_percent_encodes() {
    let encoded = discord_reactions::encode_emoji_for_discord("\u{1F440}");
    assert_eq!(encoded, "%F0%9F%91%80");
}

/// 测试勾选标记表情符号的编码
/// 勾选标记 (U+2705) 应该被正确编码
#[test]
fn encode_emoji_checkmark() {
    let encoded = discord_reactions::encode_emoji_for_discord("\u{2705}");
    assert_eq!(encoded, "%E2%9C%85");
}

/// 测试自定义公会表情符号的直通
/// 自定义表情符号格式 "name:id" 应该保持不变
#[test]
fn encode_emoji_custom_guild_emoji_passthrough() {
    let encoded = discord_reactions::encode_emoji_for_discord("custom_emoji:123456789");
    assert_eq!(encoded, "custom_emoji:123456789");
}

/// 测试简单 ASCII 字符的编码
/// 单个 ASCII 字符应该被百分比编码
#[test]
fn encode_emoji_simple_ascii_char() {
    let encoded = discord_reactions::encode_emoji_for_discord("A");
    assert_eq!(encoded, "%41");
}

/// 测试随机 Discord 确认反应来自预设池
/// 随机选择的反应应该总是来自预定义的反应池
#[test]
fn random_discord_ack_reaction_is_from_pool() {
    for _ in 0..128 {
        let emoji = discord_reactions::random_discord_ack_reaction();
        assert!(discord_reactions::discord_ack_reactions().contains(&emoji));
    }
}

/// 测试 Discord 反应 URL 的表情符号编码和前缀移除
/// 反应 URL 应该包含正确编码的表情符号
#[test]
fn discord_reaction_url_encodes_emoji_and_strips_prefix() {
    let url = discord_reactions::discord_reaction_url("123", "discord_456", "👀");
    assert_eq!(
        url,
        "https://discord.com/api/v10/channels/123/messages/456/reactions/%F0%9F%91%80/@me"
    );
}
