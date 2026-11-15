//! Lark 事件解析测试。
//!
//! 本模块验证用户 allowlist、URL challenge、文本/图片消息解析和群聊
//! mention 策略，确保 webhook 输入在进入 agent 前完成边界过滤。

use super::*;

#[test]
fn lark_user_allowed_exact() {
    let ch = make_channel();
    assert!(ch.is_user_allowed("ou_testuser123"));
    assert!(!ch.is_user_allowed("ou_other"));
}

#[test]
fn lark_user_allowed_wildcard() {
    // 通配符代表明确放开该通道用户范围，默认空列表仍保持拒绝。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    assert!(ch.is_user_allowed("ou_anyone"));
}

#[test]
fn lark_user_denied_empty() {
    let ch = LarkChannel::new("id".into(), "secret".into(), "token".into(), None, vec![], true);
    assert!(!ch.is_user_allowed("ou_anyone"));
}

#[test]
fn lark_parse_challenge() {
    let ch = make_channel();
    // URL verification 是平台握手事件，不应被转换成用户消息进入会话。
    let payload = serde_json::json!({
        "challenge": "abc123",
        "token": "test_verification_token",
        "type": "url_verification"
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_valid_text_message() {
    let ch = make_channel();
    let payload = serde_json::json!({
        "header": {
            "event_type": "im.message.receive_v1"
        },
        "event": {
            "sender": {
                "sender_id": {
                    "open_id": "ou_testuser123"
                }
            },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"Hello VibeWindow!\"}",
                "chat_id": "oc_chat123",
                "create_time": "1699999999000"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "Hello VibeWindow!");
    assert_eq!(msgs[0].sender, "ou_testuser123");
    assert_eq!(msgs[0].channel, "lark");
    assert_eq!(msgs[0].timestamp, 1_699_999_999);
}

#[test]
fn lark_parse_unauthorized_user() {
    let ch = make_channel();
    // allowlist 是入口安全边界，未授权用户消息必须在解析阶段丢弃。
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_unauthorized" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"spam\"}",
                "chat_id": "oc_chat",
                "create_time": "1000"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_image_message_uses_fallback_text() {
    // 同步解析无法下载图片时返回提示文本，避免把空内容交给后续 agent。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "image",
                "content": "{}",
                "chat_id": "oc_chat"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT);
}

#[tokio::test]
async fn lark_parse_event_payload_async_image_missing_key_uses_fallback_text() {
    // 即使走异步路径，缺少 image_key 时也不能尝试远程下载。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "image",
                "content": "{}",
                "chat_id": "oc_chat"
            }
        }
    });

    let msgs = ch.parse_event_payload_async(&payload).await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT);
}

#[test]
fn lark_parse_empty_text_skipped() {
    // 空文本没有可执行意图，跳过可减少无意义会话创建。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"\"}",
                "chat_id": "oc_chat"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_wrong_event_type() {
    let ch = make_channel();
    let payload = serde_json::json!({
        "header": { "event_type": "im.chat.disbanded_v1" },
        "event": {}
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_missing_sender() {
    // 缺少 sender 无法做 allowlist 校验，按默认拒绝处理。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"hello\"}",
                "chat_id": "oc_chat"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_unicode_message() {
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"Hello world 🌍\"}",
                "chat_id": "oc_chat",
                "create_time": "1000"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "Hello world 🌍");
}

#[test]
fn lark_parse_missing_event() {
    let ch = make_channel();
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_invalid_content_json() {
    // 平台 content 字段是嵌套 JSON 字符串，解析失败时不能猜测用户意图。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "not valid json",
                "chat_id": "oc_chat"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert!(msgs.is_empty());
}

#[test]
fn lark_parse_fallback_sender_to_open_id() {
    // 私聊缺少 chat_id 时回退到 open_id，保证仍有可回复的 sender 标识。
    let ch = LarkChannel::new(
        "id".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        true,
    );
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"hello\"}",
                "create_time": "1000"
            }
        }
    });

    let msgs = ch.parse_event_payload(&payload);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].sender, "ou_user");
}

#[test]
fn lark_parse_group_message_requires_bot_mention_when_enabled() {
    // 群聊默认需要显式 @ 当前机器人，避免机器人响应与自己无关的讨论。
    let ch = with_bot_open_id(
        LarkChannel::new(
            "cli_app123".into(),
            "secret".into(),
            "token".into(),
            None,
            vec!["*".into()],
            true,
        ),
        "ou_bot_123",
    );

    let no_mention_payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"hello\"}",
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": []
            }
        }
    });
    assert!(ch.parse_event_payload(&no_mention_payload).is_empty());

    let wrong_mention_payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"hello\"}",
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": [{ "id": { "open_id": "ou_other" } }]
            }
        }
    });
    assert!(ch.parse_event_payload(&wrong_mention_payload).is_empty());

    let bot_mention_payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"hello\"}",
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": [{ "id": { "open_id": "ou_bot_123" } }]
            }
        }
    });
    assert_eq!(ch.parse_event_payload(&bot_mention_payload).len(), 1);
}

#[test]
fn lark_parse_group_post_message_accepts_at_when_top_level_mentions_empty() {
    // 富文本 post 可能只在正文节点里携带 at，解析器需要检查内嵌结构。
    let ch = with_bot_open_id(
        LarkChannel::new(
            "cli_app123".into(),
            "secret".into(),
            "token".into(),
            None,
            vec!["*".into()],
            true,
        ),
        "ou_bot_123",
    );

    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "post",
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": [],
                "content": "{\"zh_cn\":{\"title\":\"\",\"content\":[[{\"tag\":\"at\",\"user_id\":\"ou_bot_123\",\"user_name\":\"Bot\"},{\"tag\":\"text\",\"text\":\" hi\"}]]}}"
            }
        }
    });

    assert_eq!(ch.parse_event_payload(&payload).len(), 1);
}

#[test]
fn lark_parse_group_message_allows_without_mention_when_disabled() {
    // 当调用方显式关闭 mention_only 时，群聊消息可直接进入 agent。
    let ch = LarkChannel::new(
        "cli_app123".into(),
        "secret".into(),
        "token".into(),
        None,
        vec!["*".into()],
        false,
    );

    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": "{\"text\":\"hello\"}",
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": []
            }
        }
    });

    assert_eq!(ch.parse_event_payload(&payload).len(), 1);
}
