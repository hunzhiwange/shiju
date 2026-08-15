use super::*;

use std::path::{Path, PathBuf};

/// 测试 DiscordChannel 的名称返回正确的平台标识
#[test]
fn discord_channel_name() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    assert_eq!(ch.name(), "discord");
}

#[test]
fn discord_channel_new_preserves_constructor_options() {
    let ch = DiscordChannel::new(
        "token".into(),
        Some("guild-1".into()),
        vec!["user-1".into()],
        true,
        true,
    );

    assert_eq!(ch.bot_token, "token");
    assert_eq!(ch.guild_id.as_deref(), Some("guild-1"));
    assert_eq!(ch.allowed_users, vec!["user-1"]);
    assert!(ch.listen_to_bots);
    assert!(ch.mention_only);
    assert!(ch.group_reply_allowed_sender_ids.is_empty());
    assert!(ch.workspace_dir.is_none());
}

/// 测试 Base64 解码功能能够正确解码 Bot ID
/// "MTIzNDU2" 解码后应为 "123456"
#[test]
fn base64_decode_bot_id() {
    let decoded = ids::base64_decode("MTIzNDU2");
    assert_eq!(decoded, Some("123456".to_string()));
}

/// 测试从 Bot 令牌中提取用户 ID
/// 令牌格式为: base64(user_id).timestamp.hmac
#[test]
fn bot_user_id_extraction() {
    let token = "MTIzNDU2.fake.hmac";
    let id = ids::bot_user_id_from_token(token);
    assert_eq!(id, Some("123456".to_string()));
}

/// 测试空白名单拒绝所有用户访问
/// 当白名单为空时，任何用户都不应该被允许
#[test]
fn empty_allowlist_denies_everyone() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "12345"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "anyone"));
}

/// 测试通配符 "*" 允许所有用户访问
/// 当白名单包含 "*" 时，任何用户都应该被允许
#[test]
fn wildcard_allows_everyone() {
    let ch = DiscordChannel::new("fake".into(), None, vec!["*".into()], false, false);
    assert!(permissions::is_user_allowed(&ch.allowed_users, "12345"));
    assert!(permissions::is_user_allowed(&ch.allowed_users, "anyone"));
}

/// 测试特定用户白名单的过滤功能
/// 只有在白名单中的用户才应该被允许
#[test]
fn specific_allowlist_filters() {
    let ch =
        DiscordChannel::new("fake".into(), None, vec!["111".into(), "222".into()], false, false);
    assert!(permissions::is_user_allowed(&ch.allowed_users, "111"));
    assert!(permissions::is_user_allowed(&ch.allowed_users, "222"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "333"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "unknown"));
}

/// 测试白名单是精确匹配而非子串匹配
/// 防止通过部分匹配绕过权限检查
#[test]
fn allowlist_is_exact_match_not_substring() {
    let ch = DiscordChannel::new("fake".into(), None, vec!["111".into()], false, false);
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "1111"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "11"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "0111"));
}

/// 测试空字符串用户 ID 的处理
/// 空字符串不应被白名单匹配
#[test]
fn allowlist_empty_string_user_id() {
    let ch = DiscordChannel::new("fake".into(), None, vec!["111".into()], false, false);
    assert!(!permissions::is_user_allowed(&ch.allowed_users, ""));
}

/// 测试白名单同时包含通配符和特定用户的情况
/// 通配符应该允许所有用户，特定 ID 也应该被允许
#[test]
fn allowlist_with_wildcard_and_specific() {
    let ch = DiscordChannel::new("fake".into(), None, vec!["111".into(), "*".into()], false, false);
    assert!(permissions::is_user_allowed(&ch.allowed_users, "111"));
    assert!(permissions::is_user_allowed(&ch.allowed_users, "anyone_else"));
}

/// 测试白名单的大小写敏感性
/// 用户 ID 的比较应该是大小写敏感的
#[test]
fn allowlist_case_sensitive() {
    let ch = DiscordChannel::new("fake".into(), None, vec!["ABC".into()], false, false);
    assert!(permissions::is_user_allowed(&ch.allowed_users, "ABC"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "abc"));
    assert!(!permissions::is_user_allowed(&ch.allowed_users, "Abc"));
}

/// 测试 Base64 解码空字符串
/// 空字符串应该解码为空字符串
#[test]
fn base64_decode_empty_string() {
    let decoded = ids::base64_decode("");
    assert_eq!(decoded, Some(String::new()));
}

/// 测试 Base64 解码无效字符
/// 包含无效 Base64 字符的输入应该返回 None
#[test]
fn base64_decode_invalid_chars() {
    let decoded = ids::base64_decode("!!!!");
    assert!(decoded.is_none());
}

/// 测试从空令牌中提取用户 ID
/// 空令牌应该返回空字符串
#[test]
fn bot_user_id_from_empty_token() {
    let id = ids::bot_user_id_from_token("");
    assert_eq!(id, Some(String::new()));
}

/// 测试 Bot 提及检测支持普通和昵称格式
/// Discord 支持两种提及格式: <@12345> 和 <@!12345>
#[test]
fn contains_bot_mention_supports_plain_and_nick_forms() {
    assert!(content::contains_bot_mention("hi <@12345>", "12345"));
    assert!(content::contains_bot_mention("hi <@!12345>", "12345"));
    assert!(!content::contains_bot_mention("hi <@99999>", "12345"));
}

/// 测试当需要提及但消息中没有提及时的处理
/// 如果启用了提及要求，没有提及的消息应返回 None
#[test]
fn normalize_incoming_content_requires_mention_when_enabled() {
    let cleaned = content::normalize_incoming_content("hello there", true, "12345");
    assert!(cleaned.is_none());
}

/// 测试消息内容规范化能够移除提及并修剪空白
/// 提及标记应该被移除，消息应该被修剪
#[test]
fn normalize_incoming_content_strips_mentions_and_trims() {
    let cleaned = content::normalize_incoming_content("  <@!12345> run status  ", true, "12345");
    assert_eq!(cleaned.as_deref(), Some("run status"));
}

/// 测试规范化后空消息的拒绝
/// 如果移除提及后消息为空，应该返回 None
#[test]
fn normalize_incoming_content_rejects_empty_after_strip() {
    let cleaned = content::normalize_incoming_content("<@12345>", true, "12345");
    assert!(cleaned.is_none());
}

/// 测试群组回复允许发送者 ID 列表的规范化
/// 应该修剪空白、去重并移除空项
#[test]
fn normalize_group_reply_allowed_sender_ids_trims_and_deduplicates() {
    let normalized = content::normalize_group_reply_allowed_sender_ids(vec![
        " 111 ".into(),
        "111".into(),
        String::new(),
        "  ".into(),
        "222".into(),
    ]);
    assert_eq!(normalized, vec!["111".to_string(), "222".to_string()]);
}

/// 测试群组回复发送者覆盖的精确匹配和通配符
/// 应该支持特定 ID 和通配符 "*" 的组合
#[test]
fn group_reply_sender_override_matches_exact_and_wildcard() {
    let ch = DiscordChannel::new("token".into(), None, vec!["*".into()], false, true)
        .with_group_reply_allowed_senders(vec!["111".into(), "*".into()]);

    assert!(permissions::is_group_sender_trigger_enabled(
        &ch.group_reply_allowed_sender_ids,
        "111"
    ));
    assert!(permissions::is_group_sender_trigger_enabled(
        &ch.group_reply_allowed_sender_ids,
        "anyone"
    ));
    assert!(!permissions::is_group_sender_trigger_enabled(&ch.group_reply_allowed_sender_ids, ""));
}

/// 测试 with_workspace_dir 设置工作区目录字段
/// 构建器方法应该正确设置工作区目录
#[test]
fn with_workspace_dir_sets_field() {
    let channel = DiscordChannel::new("fake".into(), None, vec![], false, false)
        .with_workspace_dir(PathBuf::from("/tmp/discord-workspace"));
    assert_eq!(channel.workspace_dir.as_deref(), Some(Path::new("/tmp/discord-workspace")));
}
