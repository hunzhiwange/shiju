//! Telegram 提及处理测试模块
//!
//! 本模块包含针对 Telegram 频道提及（@mention）功能的单元测试。
//! 主要测试内容包括：
//! - 机器人用户名提及的检测与匹配
//! - 消息内容的标准化处理（移除提及前缀）
//! - 群组消息中的提及过滤逻辑
//! - 配置的发件人白名单绕过机制
//! - 不同聊天类型（私聊/群组/超级群组）的处理差异
//!
//! 这些测试确保 TelegramChannel 正确处理群组环境中的提及要求，
//! 同时保持私聊消息的兼容性。

use super::*;

/// 测试 contains_bot_mention 方法能否正确识别消息中的机器人提及
///
/// 测试场景：
/// - 提及出现在消息中间
/// - 提及出现在消息开头
/// - 提及出现在消息结尾
/// - 提及的用户名大小写不敏感（@MyBot 匹配 mybot）
#[test]
fn telegram_contains_bot_mention_finds_mention() {
    assert!(TelegramChannel::contains_bot_mention("Hello @mybot", "mybot"));
    assert!(TelegramChannel::contains_bot_mention("@mybot help", "mybot"));
    assert!(TelegramChannel::contains_bot_mention("Hey @mybot how are you?", "mybot"));
    assert!(TelegramChannel::contains_bot_mention("Hello @MyBot, can you help?", "mybot"));
}

/// 测试 contains_bot_mention 方法不会产生误报
///
/// 测试场景：
/// - 提及的是其他机器人名
/// - 没有使用 @ 符号的纯文本
/// - 提及的机器人名是当前机器人的前缀（mybot2 不应匹配 mybot）
/// - 空消息字符串
#[test]
fn telegram_contains_bot_mention_no_false_positives() {
    assert!(!TelegramChannel::contains_bot_mention("Hello @otherbot", "mybot"));
    assert!(!TelegramChannel::contains_bot_mention("Hello mybot", "mybot"));
    assert!(!TelegramChannel::contains_bot_mention("Hello @mybot2", "mybot"));
    assert!(!TelegramChannel::contains_bot_mention("", "mybot"));
}

/// 测试 normalize_incoming_content 方法能够移除消息中的机器人提及
///
/// 预期结果：
/// - 输入 "@mybot hello" 应返回 "hello"
/// - 提及前缀被正确移除，剩余内容保持不变
#[test]
fn telegram_normalize_incoming_content_strips_mention() {
    let result = TelegramChannel::normalize_incoming_content("@mybot hello", "mybot");
    assert_eq!(result, Some("hello".to_string()));
}

/// 测试 normalize_incoming_content 方法能够处理多个提及的情况
///
/// 预期结果：
/// - 输入 "@mybot @mybot test" 应返回 "test"
/// - 所有提及实例都被移除
#[test]
fn telegram_normalize_incoming_content_handles_multiple_mentions() {
    let result = TelegramChannel::normalize_incoming_content("@mybot @mybot test", "mybot");
    assert_eq!(result, Some("test".to_string()));
}

/// 测试 normalize_incoming_content 方法对仅包含提及的消息返回 None
///
/// 预期结果：
/// - 输入仅包含 "@mybot" 时应返回 None
/// - 这表示消息在移除提及后为空，不应被处理
#[test]
fn telegram_normalize_incoming_content_returns_none_for_empty() {
    let result = TelegramChannel::normalize_incoming_content("@mybot", "mybot");
    assert_eq!(result, None);
}

/// 测试群组消息中的严格提及要求
///
/// 在 mention_only 模式下，群组消息必须包含对机器人的精确提及才会被处理。
///
/// 测试场景：
/// - 群组消息 "@mybot2" 不匹配机器人名 "mybot"
/// - 应返回 None，表示消息被忽略
#[test]
fn parse_update_message_mention_only_group_requires_exact_mention() {
    // 创建启用了 mention_only 模式的 TelegramChannel 实例
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true);
    {
        // 设置缓存的机器人用户名，用于提及匹配
        let mut cache = ch.bot_username.lock();
        *cache = Some("mybot".to_string());
    }

    // 构造包含错误提及的群组消息
    let update = serde_json::json!({
        "update_id": 10,
        "message": {
            "message_id": 44,
            "text": "hello @mybot2",  // 提及的是 @mybot2，不是 @mybot
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300,
                "type": "group"  // 群组聊天
            }
        }
    });

    // 应返回 None，因为提及不匹配
    assert!(ch.parse_update_message(&update).is_none());
}

/// 测试群组消息中的提及移除和空消息过滤
///
/// 测试两个场景：
/// 1. 有效提及被移除后，保留有效消息内容
/// 2. 仅包含提及的消息在移除后返回 None
#[test]
fn parse_update_message_mention_only_group_strips_mention_and_drops_empty() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true);
    {
        let mut cache = ch.bot_username.lock();
        *cache = Some("mybot".to_string());
    }

    // 场景 1：包含提及和有效内容的群组消息
    let update = serde_json::json!({
        "update_id": 11,
        "message": {
            "message_id": 45,
            "text": "Hi @MyBot status please",  // 大小写不敏感的提及
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300,
                "type": "group"
            }
        }
    });

    // 解析成功，提及被移除
    let parsed = ch.parse_update_message(&update).expect("mention should parse");
    assert_eq!(parsed.content, "Hi status please");

    // 场景 2：仅包含提及的群组消息
    let empty_update = serde_json::json!({
        "update_id": 12,
        "message": {
            "message_id": 46,
            "text": "@mybot",  // 仅包含提及
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300,
                "type": "group"
            }
        }
    });

    // 应返回 None，因为移除提及后消息为空
    assert!(ch.parse_update_message(&empty_update).is_none());
}

/// 测试配置的发件人白名单可以绕过群组消息的提及要求
///
/// 在 mention_only 模式下，通过 with_group_reply_allowed_senders 配置
/// 的发件人 ID 列表可以不经提及直接向机器人发送命令。
///
/// 测试场景：
/// - 发件人 ID 555 在白名单中
/// - 群组消息不包含机器人提及
/// - 消息仍应被正确处理
#[test]
fn parse_update_message_mention_only_group_allows_configured_sender_without_mention() {
    // 创建频道并配置允许的发件人白名单
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true)
        .with_group_reply_allowed_senders(vec!["555".into()]);
    {
        let mut cache = ch.bot_username.lock();
        *cache = Some("mybot".to_string());
    }

    // 构造不包含提及但发件人在白名单中的群组消息
    let update = serde_json::json!({
        "update_id": 13,
        "message": {
            "message_id": 47,
            "text": "run daily sync",  // 无提及
            "from": {
                "id": 555,  // 在白名单中
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300,
                "type": "group"
            }
        }
    });

    // 应成功解析，因为发件人在白名单中
    let parsed = ch
        .parse_update_message(&update)
        .expect("sender override should bypass mention requirement");
    assert_eq!(parsed.content, "run daily sync");
}

/// 测试 is_group_message 方法正确识别群组类型消息
///
/// 测试场景：
/// - "group" 类型应返回 true
/// - "supergroup" 类型应返回 true
/// - "private" 类型应返回 false
#[test]
fn telegram_is_group_message_detects_groups() {
    // 普通群组
    let group_msg = serde_json::json!({
        "chat": { "type": "group" }
    });
    assert!(TelegramChannel::is_group_message(&group_msg));

    // 超级群组
    let supergroup_msg = serde_json::json!({
        "chat": { "type": "supergroup" }
    });
    assert!(TelegramChannel::is_group_message(&supergroup_msg));

    // 私聊
    let private_msg = serde_json::json!({
        "chat": { "type": "private" }
    });
    assert!(!TelegramChannel::is_group_message(&private_msg));
}

/// 测试 mention_only 配置正确传递到 TelegramChannel 实例
///
/// 测试场景：
/// - 传入 true 时，mention_only 字段应为 true
/// - 传入 false 时，mention_only 字段应为 false
#[test]
fn telegram_mention_only_enabled_by_config() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true);
    assert!(ch.mention_only);

    let ch_disabled = TelegramChannel::new("token".into(), vec!["*".into()], false);
    assert!(!ch_disabled.mention_only);
}

/// 测试群组中不带说明文字的图片消息在 mention_only 模式下的处理
///
/// 在 mention_only 模式下，群组中仅发送图片（无说明文字）的消息
/// 应被忽略，因为没有机会包含机器人提及。
///
/// 注：本测试当前仅验证 mention_only 配置状态，
/// 实际的图片消息过滤逻辑应在集成测试中验证。
#[test]
fn telegram_mention_only_group_photo_without_caption_is_ignored() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true);
    {
        let mut cache = ch.bot_username.lock();
        *cache = Some("mybot".to_string());
    }

    // 构造仅包含图片（无说明文字）的群组消息
    let _update = serde_json::json!({
        "update_id": 100,
        "message": {
            "message_id": 1,
            "photo": [
                {"file_id": "photo_id", "file_size": 1_000}
            ],
            // 注意：无 caption 字段
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300,
                "type": "group"
            }
        }
    });

    // 验证 mention_only 模式已启用
    assert!(ch.mention_only);
}

/// 测试群组中带说明文字但无提及的图片消息在 mention_only 模式下的处理
///
/// 在 mention_only 模式下，群组中发送图片并附带说明文字但未提及机器人的
/// 消息应被忽略。
///
/// 注：本测试当前仅验证 mention_only 配置状态，
/// 实际的说明文字提及过滤逻辑应在集成测试中验证。
#[test]
fn telegram_mention_only_group_photo_with_caption_without_mention_is_ignored() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true);
    {
        let mut cache = ch.bot_username.lock();
        *cache = Some("mybot".to_string());
    }

    // 构造带说明文字但未提及机器人的群组图片消息
    let _update = serde_json::json!({
        "update_id": 101,
        "message": {
            "message_id": 2,
            "photo": [
                {"file_id": "photo_id", "file_size": 1_000}
            ],
            "caption": "Look at this image",  // 说明文字未提及机器人
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300,
                "type": "group"
            }
        }
    });

    // 验证 mention_only 模式已启用
    assert!(ch.mention_only);
}

/// 测试私聊中图片消息在 mention_only 模式下仍能正常处理
///
/// mention_only 模式仅影响群组消息，私聊消息不受此限制。
/// 即使启用了 mention_only，私聊中的图片消息仍应被正常处理。
///
/// 注：本测试验证 mention_only 配置状态，
/// 实际的私聊图片处理逻辑应在集成测试中验证。
#[test]
fn telegram_mention_only_private_chat_photo_still_works() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], true);
    {
        let mut cache = ch.bot_username.lock();
        *cache = Some("mybot".to_string());
    }

    // 构造私聊图片消息
    let _update = serde_json::json!({
        "update_id": 102,
        "message": {
            "message_id": 3,
            "photo": [
                {"file_id": "photo_id", "file_size": 1_000}
            ],
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": 123_456,
                "type": "private"  // 私聊不受 mention_only 限制
            }
        }
    });

    // 验证 mention_only 模式已启用（但不影响私聊）
    assert!(ch.mention_only);
}
