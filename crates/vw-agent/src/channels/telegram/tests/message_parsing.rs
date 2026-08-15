//! Telegram 消息解析测试模块
//!
//! 本模块包含对 TelegramChannel 消息解析功能的单元测试，
//! 涵盖消息更新解析、发送者信息提取、回复上下文提取等核心功能。

use super::*;

/// 测试消息更新解析是否使用聊天 ID 作为回复目标
///
/// 验证点：
/// - 从 Telegram 更新中正确解析发送者用户名
/// - 使用聊天 ID 作为回复目标（而非发送者 ID）
/// - 正确提取消息内容
/// - 生成符合格式的消息 ID
#[test]
fn parse_update_message_uses_chat_id_as_reply_target() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], false);
    let update = serde_json::json!({
        "update_id": 1,
        "message": {
            "message_id": 33,
            "text": "hello",
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300
            }
        }
    });

    let msg = ch.parse_update_message(&update).expect("message should parse");

    assert_eq!(msg.sender, "alice");
    assert_eq!(msg.reply_target, "-100200300");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.id, "telegram_-100200300_33");
}

/// 测试允许列表支持数字 ID（用户无用户名的情况）
///
/// 验证点：
/// - 当用户没有设置用户名时，使用数字 ID 作为身份标识
/// - 数字 ID 允许列表能正常工作
/// - 正确处理发送者信息提取
#[test]
fn parse_update_message_allows_numeric_id_without_username() {
    let ch = TelegramChannel::new("token".into(), vec!["555".into()], false);
    let update = serde_json::json!({
        "update_id": 2,
        "message": {
            "message_id": 9,
            "text": "ping",
            "from": {
                "id": 555
            },
            "chat": {
                "id": 12345
            }
        }
    });

    let msg = ch.parse_update_message(&update).expect("numeric allowlist should pass");

    assert_eq!(msg.sender, "555");
    assert_eq!(msg.reply_target, "12345");
}

/// 测试论坛主题模式下提取线程 ID
///
/// 验证点：
/// - 正确提取 message_thread_id 字段
/// - 回复目标格式为 "chat_id:thread_id"
/// - 适用于 Telegram 群组话题功能
#[test]
fn parse_update_message_extracts_thread_id_for_forum_topic() {
    let ch = TelegramChannel::new("token".into(), vec!["*".into()], false);
    let update = serde_json::json!({
        "update_id": 3,
        "message": {
            "message_id": 42,
            "text": "hello from topic",
            "from": {
                "id": 555,
                "username": "alice"
            },
            "chat": {
                "id": -100_200_300
            },
            "message_thread_id": 789
        }
    });

    let msg = ch.parse_update_message(&update).expect("message with thread_id should parse");

    assert_eq!(msg.sender, "alice");
    assert_eq!(msg.reply_target, "-100200300:789");
    assert_eq!(msg.content, "hello from topic");
    assert_eq!(msg.id, "telegram_-100200300_42");
}

/// 测试解析消息时包含回复上下文
///
/// 验证点：
/// - 正确提取被回复消息的内容
/// - 回复上下文以引用格式呈现（> @username: > content）
/// - 用户消息内容与引用内容正确组合
#[test]
fn parse_update_message_includes_reply_context() {
    let ch = TelegramChannel::new("t".into(), vec!["*".into()], false);
    let update = serde_json::json!({
        "message": {
            "message_id": 10,
            "text": "translate this",
            "from": { "id": 1, "username": "alice" },
            "chat": { "id": 100, "type": "private" },
            "reply_to_message": {
                "from": { "username": "bot" },
                "text": "Bonjour le monde"
            }
        }
    });
    let parsed = ch.parse_update_message(&update).unwrap();
    assert!(
        parsed.content.starts_with("> @bot:"),
        "content should start with quote: {}",
        parsed.content
    );
    assert!(parsed.content.contains("translate this"), "content should contain user text");
    assert!(parsed.content.contains("Bonjour le monde"), "content should contain quoted text");
}

/// 测试提取发送者信息（有用户名的情况）
///
/// 验证点：
/// - 正确提取 username 字段
/// - 正确提取发送者 ID
/// - 身份标识优先使用用户名
#[test]
fn extract_sender_info_with_username() {
    let msg = serde_json::json!({
        "from": { "id": 123, "username": "alice" }
    });
    let (username, sender_id, identity) = TelegramChannel::extract_sender_info(&msg);
    assert_eq!(username, "alice");
    assert_eq!(sender_id, Some("123".to_string()));
    assert_eq!(identity, "alice");
}

/// 测试提取发送者信息（无用户名的情况）
///
/// 验证点：
/// - 用户名默认为 "unknown"
/// - 正确提取发送者数字 ID
/// - 身份标识使用数字 ID
#[test]
fn extract_sender_info_without_username() {
    let msg = serde_json::json!({
        "from": { "id": 42 }
    });
    let (username, sender_id, identity) = TelegramChannel::extract_sender_info(&msg);
    assert_eq!(username, "unknown");
    assert_eq!(sender_id, Some("42".to_string()));
    assert_eq!(identity, "42");
}

/// 测试提取回复上下文（文本消息）
///
/// 验证点：
/// - 正确提取被回复的文本消息
/// - 引用格式为 "> @username:\n> text"
#[test]
fn extract_reply_context_text_message() {
    let ch = TelegramChannel::new("t".into(), vec!["*".into()], false);
    let msg = serde_json::json!({
        "reply_to_message": {
            "from": { "username": "alice" },
            "text": "Hello world"
        }
    });
    let ctx = ch.extract_reply_context(&msg).unwrap();
    assert_eq!(ctx, "> @alice:\n> Hello world");
}

/// 测试提取回复上下文（语音消息）
///
/// 验证点：
/// - 语音消息转换为 "[Voice message]" 占位符
/// - 引用格式正确
#[test]
fn extract_reply_context_voice_message() {
    let ch = TelegramChannel::new("t".into(), vec!["*".into()], false);
    let msg = serde_json::json!({
        "reply_to_message": {
            "from": { "username": "bob" },
            "voice": { "file_id": "abc", "duration": 5 }
        }
    });
    let ctx = ch.extract_reply_context(&msg).unwrap();
    assert_eq!(ctx, "> @bob:\n> [Voice message]");
}

/// 测试提取回复上下文（无回复消息）
///
/// 验证点：
/// - 当消息不是回复时，返回 None
#[test]
fn extract_reply_context_no_reply() {
    let ch = TelegramChannel::new("t".into(), vec!["*".into()], false);
    let msg = serde_json::json!({
        "text": "just a regular message"
    });
    assert!(ch.extract_reply_context(&msg).is_none());
}

/// 测试提取回复上下文（无用户名时使用名）
///
/// 验证点：
/// - 当发送者无 username 时，回退到 first_name
/// - 引用格式正确
#[test]
fn extract_reply_context_no_username_uses_first_name() {
    let ch = TelegramChannel::new("t".into(), vec!["*".into()], false);
    let msg = serde_json::json!({
        "reply_to_message": {
            "from": { "id": 999, "first_name": "Charlie" },
            "text": "Hi there"
        }
    });
    let ctx = ch.extract_reply_context(&msg).unwrap();
    assert_eq!(ctx, "> @Charlie:\n> Hi there");
}
