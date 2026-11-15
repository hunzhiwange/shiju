//! Nextcloud Talk 通道测试模块
//!
//! 本模块包含 `NextcloudTalkChannel` 的单元测试，覆盖以下功能：
//!
//! - 通道名称获取
//! - 用户白名单验证（精确匹配与通配符）
//! - Webhook 载荷解析（传统格式与 Activity Streams 格式）
//! - 消息过滤（非消息事件、机器人消息、系统消息、未授权发送者）
//! - HMAC-SHA256 签名验证

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 创建默认的 NextcloudTalkChannel 测试实例
    ///
    /// # 返回值
    ///
    /// 返回一个配置了以下参数的 `NextcloudTalkChannel`：
    /// - 服务器地址：`https://cloud.example.com`
    /// - 应用令牌：`app-token`
    /// - 用户白名单：`["user_a"]`
    fn make_channel() -> NextcloudTalkChannel {
        NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["user_a".into()],
        )
    }

    /// 测试通道名称返回正确的标识符
    ///
    /// 验证 `name()` 方法返回 `"nextcloud_talk"` 字符串。
    #[test]
    fn nextcloud_talk_channel_name() {
        let channel = make_channel();
        assert_eq!(channel.name(), "nextcloud_talk");
    }

    /// 测试用户白名单的精确匹配与通配符行为
    ///
    /// 验证以下场景：
    /// - 白名单中的用户 `user_a` 应被允许
    /// - 不在白名单中的用户 `user_b` 应被拒绝
    /// - 使用通配符 `"*"` 时，任意用户都应被允许
    #[test]
    fn nextcloud_talk_user_allowlist_exact_and_wildcard() {
        let channel = make_channel();
        assert!(channel.is_user_allowed("user_a"));
        assert!(!channel.is_user_allowed("user_b"));

        let wildcard = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["*".into()],
        );
        assert!(wildcard.is_user_allowed("any_user"));
    }

    /// 测试解析有效的传统格式消息载荷
    ///
    /// 验证能够正确解析包含完整字段的传统 Nextcloud Talk webhook 载荷：
    /// - `type`: 必须为 `"message"`
    /// - `object.token`: 用作回复目标（房间令牌）
    /// - `message.id`: 消息唯一标识符
    /// - `message.actorId`: 发送者用户名
    /// - `message.timestamp`: Unix 时间戳
    /// - `message.message`: 消息正文内容
    #[test]
    fn nextcloud_talk_parse_valid_message_payload() {
        let channel = make_channel();
        let payload = serde_json::json!({
            "type": "message",
            "object": {
                "id": "42",
                "token": "room-token-123",
                "name": "Team Room",
                "type": "room"
            },
            "message": {
                "id": 77,
                "token": "room-token-123",
                "actorType": "users",
                "actorId": "user_a",
                "actorDisplayName": "User A",
                "timestamp": 1_735_701_200,
                "messageType": "comment",
                "systemMessage": "",
                "message": "Hello from Nextcloud"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, "77");
        assert_eq!(messages[0].reply_target, "room-token-123");
        assert_eq!(messages[0].sender, "user_a");
        assert_eq!(messages[0].content, "Hello from Nextcloud");
        assert_eq!(messages[0].channel, "nextcloud_talk");
        assert_eq!(messages[0].timestamp, 1_735_701_200);
    }

    /// 测试跳过非消息类型的事件
    ///
    /// 当载荷的 `type` 字段不是 `"message"` 时（如 `"room"`），
    /// 解析器应返回空列表，不产生任何消息。
    #[test]
    fn nextcloud_talk_parse_skips_non_message_events() {
        let channel = make_channel();
        let payload = serde_json::json!({
            "type": "room",
            "object": {"token": "room-token-123"},
            "message": {
                "actorType": "users",
                "actorId": "user_a",
                "message": "Hello"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert!(messages.is_empty());
    }

    /// 测试跳过机器人发送的消息
    ///
    /// 当消息的 `actorType` 为 `"bots"` 时，解析器应忽略该消息，
    /// 避免机器人消息形成无限循环。
    #[test]
    fn nextcloud_talk_parse_skips_bot_messages() {
        let channel = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["*".into()],
        );
        let payload = serde_json::json!({
            "type": "message",
            "object": {"token": "room-token-123"},
            "message": {
                "actorType": "bots",
                "actorId": "bot_1",
                "message": "Self message"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert!(messages.is_empty());
    }

    /// 测试解析 Activity Streams 格式的 Create Note 载荷
    ///
    /// Activity Streams 是 Nextcloud Talk 支持的另一种 webhook 格式，
    /// 本测试验证能够正确解析：
    /// - `type`: `"Create"`
    /// - `actor.type`: `"Person"`
    /// - `object.type`: `"Note"`
    /// - `object.content`: JSON 字符串，包含实际消息内容
    /// - `target.id`: 房间令牌
    #[test]
    fn nextcloud_talk_parse_activity_streams_create_note_payload() {
        let channel = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["test".into()],
        );

        let payload = serde_json::json!({
            "type": "Create",
            "actor": {
                "type": "Person",
                "id": "users/test",
                "name": "test"
            },
            "object": {
                "type": "Note",
                "id": "177",
                "content": "{\"message\":\"hello\",\"parameters\":[]}",
                "mediaType": "text/markdown"
            },
            "target": {
                "type": "Collection",
                "id": "yyrubgfp",
                "name": "TESTCHAT"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, "177");
        assert_eq!(messages[0].reply_target, "yyrubgfp");
        assert_eq!(messages[0].sender, "test");
        assert_eq!(messages[0].content, "hello");
    }

    /// 测试跳过 Application 类型发送者的 Activity Streams 消息
    ///
    /// 当 Activity Streams 载荷的 `actor.type` 为 `"Application"` 时，
    /// 表示消息来自应用程序而非真实用户，应被忽略以防止循环。
    #[test]
    fn nextcloud_talk_parse_activity_streams_skips_application_actor() {
        let channel = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["*".into()],
        );

        let payload = serde_json::json!({
            "type": "Create",
            "actor": {
                "type": "Application",
                "id": "apps/vibewindow"
            },
            "object": {
                "type": "Note",
                "id": "178",
                "content": "{\"message\":\"ignore me\"}"
            },
            "target": {
                "id": "yyrubgfp"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert!(messages.is_empty());
    }

    /// 测试白名单同时匹配完整 actor ID 和简短用户名
    ///
    /// 当白名单配置为完整 actor ID（如 `"users/test"`）时，
    /// `is_user_allowed` 应同时接受：
    /// - 完整格式：`"users/test"`
    /// - 简短格式：`"test"`
    #[test]
    fn nextcloud_talk_allowlist_matches_full_and_short_actor_ids() {
        let channel = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["users/test".into()],
        );
        assert!(channel.is_user_allowed("users/test"));
        assert!(channel.is_user_allowed("test"));
    }

    /// 测试跳过未授权发送者的消息
    ///
    /// 当消息发送者不在白名单中时，解析器应返回空列表，
    /// 确保只有授权用户的消息被处理。
    #[test]
    fn nextcloud_talk_parse_skips_unauthorized_sender() {
        let channel = make_channel();
        let payload = serde_json::json!({
            "type": "message",
            "object": {"token": "room-token-123"},
            "message": {
                "actorType": "users",
                "actorId": "user_b",
                "message": "Unauthorized"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert!(messages.is_empty());
    }

    /// 测试跳过系统消息
    ///
    /// 当消息的 `systemMessage` 字段非空时（如 `"joined"`），
    /// 表示这是一条系统通知而非用户消息，应被忽略。
    #[test]
    fn nextcloud_talk_parse_skips_system_message() {
        let channel = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["*".into()],
        );
        let payload = serde_json::json!({
            "type": "message",
            "object": {"token": "room-token-123"},
            "message": {
                "actorType": "users",
                "actorId": "user_a",
                "messageType": "comment",
                "systemMessage": "joined",
                "message": ""
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert!(messages.is_empty());
    }

    /// 测试时间戳从毫秒转换为秒
    ///
    /// Nextcloud Talk 可能发送毫秒级时间戳，解析器应将其转换为秒级，
    /// 通过截断（而非四舍五入）去除毫秒部分。
    #[test]
    fn nextcloud_talk_parse_timestamp_millis_to_seconds() {
        let channel = NextcloudTalkChannel::new(
            "https://cloud.example.com".into(),
            "app-token".into(),
            vec!["*".into()],
        );
        let payload = serde_json::json!({
            "type": "message",
            "object": {"token": "room-token-123"},
            "message": {
                "actorType": "users",
                "actorId": "user_a",
                "timestamp": 1_735_701_200_123_u64,
                "message": "hello"
            }
        });

        let messages = channel.parse_webhook_payload(&payload);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].timestamp, 1_735_701_200);
    }

    /// 测试用的 Webhook 签名密钥常量
    ///
    /// 用于签名验证测试的固定密钥值。
    const TEST_WEBHOOK_SECRET: &str = "nextcloud_test_webhook_secret";

    /// 测试有效签名的验证通过
    ///
    /// 验证流程：
    /// 1. 构造载荷：`{random}{body}`
    /// 2. 使用 HMAC-SHA256 计算签名
    /// 3. 将签名转换为十六进制字符串
    /// 4. 验证函数应返回 `true`
    #[test]
    fn nextcloud_talk_signature_verification_valid() {
        let secret = TEST_WEBHOOK_SECRET;
        let random = "random-seed";
        let body = r#"{"type":"message"}"#;

        let payload = format!("{random}{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_nextcloud_talk_signature(secret, random, body, &signature));
    }

    /// 测试无效签名被正确拒绝
    ///
    /// 当提供的签名与实际计算结果不匹配时，
    /// 验证函数应返回 `false`。
    #[test]
    fn nextcloud_talk_signature_verification_invalid() {
        assert!(!verify_nextcloud_talk_signature(
            TEST_WEBHOOK_SECRET,
            "random-seed",
            r#"{"type":"message"}"#,
            "deadbeef"
        ));
    }

    /// 测试签名验证接受带 `sha256=` 前缀的签名
    ///
    /// 某些客户端可能在签名前添加 `sha256=` 前缀，
    /// 验证函数应能正确处理这种格式。
    #[test]
    fn nextcloud_talk_signature_verification_accepts_sha256_prefix() {
        let secret = TEST_WEBHOOK_SECRET;
        let random = "random-seed";
        let body = r#"{"type":"message"}"#;

        let payload = format!("{random}{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verify_nextcloud_talk_signature(secret, random, body, &signature));
    }
}
