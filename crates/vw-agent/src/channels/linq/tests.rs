//! Linq 通道单元测试模块
//!
//! 本模块包含针对 `LinqChannel` 实现的全面测试用例，覆盖以下功能领域：
//!
//! - **通道基础功能**：通道名称获取、电话号码访问器
//! - **发送者权限验证**：精确匹配、通配符、空列表场景
//! - **Webhook 载荷解析**：
//!   - 有效的文本消息解析
//!   - v3 版本 API 载荷格式
//!   - 发送者电话号码规范化（自动添加 `+` 前缀）
//!   - 多部分文本消息合并
//!   - 媒体消息处理（图片/非图片）
//!   - 回复目标回退逻辑
//! - **消息过滤**：
//!   - 跳过自己发送的消息（`is_from_me` / `is_me` / `outbound`）
//!   - 跳过非消息事件
//!   - 过滤未授权发送者
//!   - 跳过空文本内容
//! - **签名验证**：
//!   - 有效签名验证
//!   - 无效签名拒绝
//!   - 过期时间戳拒绝
//!   - 支持 `sha256=` 前缀格式
//!   - 支持大写十六进制格式

use super::*;

/// 内部测试模块
///
/// 使用 `#[allow(dead_code)]` 标注以允许未使用的代码警告，
/// 因为测试模块中的某些辅助函数可能在特定配置下未被调用。
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 创建用于测试的 LinqChannel 实例
    ///
    /// # 返回值
    ///
    /// 返回一个预配置的 `LinqChannel` 实例，包含以下固定配置：
    /// - 令牌：`"test-token"`
    /// - 电话号码：`"+15551234567"`
    /// - 允许的发送者列表：`["+1234567890"]`
    ///
    /// # 用途
    ///
    /// 该辅助函数用于创建具有标准配置的测试通道实例，
    /// 避免在每个测试用例中重复相同的构造代码。
    fn make_channel() -> LinqChannel {
        LinqChannel::new("test-token".into(), "+15551234567".into(), vec!["+1234567890".into()])
    }

    /// 测试通道名称返回是否正确
    ///
    /// 验证 `LinqChannel::name()` 方法返回字符串 `"linq"`。
    #[test]
    fn linq_channel_name() {
        let ch = make_channel();
        assert_eq!(ch.name(), "linq");
    }

    /// 测试发送者权限验证 - 精确匹配
    ///
    /// 验证场景：
    /// - 允许列表中精确匹配的电话号码应被允许
    /// - 不在允许列表中的电话号码应被拒绝
    #[test]
    fn linq_sender_allowed_exact() {
        let ch = make_channel();
        // 允许列表中的号码应通过验证
        assert!(ch.is_sender_allowed("+1234567890"));
        // 不在允许列表中的号码应被拒绝
        assert!(!ch.is_sender_allowed("+9876543210"));
    }

    /// 测试发送者权限验证 - 通配符模式
    ///
    /// 当允许列表包含通配符 `"*"` 时，所有发送者都应被允许。
    #[test]
    fn linq_sender_allowed_wildcard() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 通配符模式下任意号码都应被允许
        assert!(ch.is_sender_allowed("+1234567890"));
        assert!(ch.is_sender_allowed("+9999999999"));
    }

    /// 测试发送者权限验证 - 空允许列表
    ///
    /// 当允许列表为空时，所有发送者都应被拒绝。
    /// 这是一个安全默认行为：无明确授权则拒绝访问。
    #[test]
    fn linq_sender_allowed_empty() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec![]);
        // 空允许列表时应拒绝所有发送者
        assert!(!ch.is_sender_allowed("+1234567890"));
    }

    /// 测试解析有效的文本消息 - 旧版载荷格式
    ///
    /// 验证对旧版 Linq API v3 载荷格式的解析能力：
    /// - 正确提取发送者号码
    /// - 正确提取文本内容
    /// - 正确设置通道标识
    /// - 正确提取回复目标（chat_id）
    #[test]
    fn linq_parse_valid_text_message() {
        let ch = make_channel();
        // 构造旧版 v3 格式的 webhook 载荷
        let payload = serde_json::json!({
            "api_version": "v3",
            "event_type": "message.received",
            "event_id": "evt-123",
            "created_at": "2025-01-15T12:00:00Z",
            "trace_id": "trace-456",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "recipient_phone": "+15551234567",
                "is_from_me": false,
                "service": "iMessage",
                "message": {
                    "id": "msg-abc",
                    "parts": [{
                        "type": "text",
                        "value": "Hello VibeWindow!"
                    }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 验证解析结果
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
        assert_eq!(msgs[0].content, "Hello VibeWindow!");
        assert_eq!(msgs[0].channel, "linq");
        assert_eq!(msgs[0].reply_target, "chat-789");
    }

    /// 测试解析当前 v3 载荷格式
    ///
    /// 验证对当前 Linq API v3 载荷格式的解析能力。
    /// 新格式使用 `chat.id`、`sender_handle.handle`、`sender_handle.is_me` 和 `direction` 字段。
    #[test]
    fn linq_parse_current_v3_payload_shape() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造当前 v3 格式的 webhook 载荷
        let payload = serde_json::json!({
            "api_version": "v3",
            "event_type": "message.received",
            "created_at": "2026-02-25T19:00:00Z",
            "data": {
                "chat": {
                    "id": "chat-v3-123"
                },
                "sender_handle": {
                    "handle": "+12197797846",
                    "is_me": false
                },
                "direction": "inbound",
                "parts": [{
                    "type": "text",
                    "value": "hi clawd ppp"
                }]
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 验证解析结果
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+12197797846");
        assert_eq!(msgs[0].content, "hi clawd ppp");
        assert_eq!(msgs[0].reply_target, "chat-v3-123");
    }

    /// 测试跳过出站/自己发送的消息 - v3 格式
    ///
    /// 验证对于 v3 格式中标记为 `is_me: true` 或 `direction: "outbound"` 的消息，
    /// 解析器应返回空列表，避免处理自己发送的消息。
    #[test]
    fn linq_parse_current_v3_outbound_is_skipped() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造出站消息载荷（自己发送的消息）
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat": { "id": "chat-v3-123" },
                "sender_handle": {
                    "handle": "+12197797846",
                    "is_me": true
                },
                "direction": "outbound",
                "parts": [{ "type": "text", "value": "self echo" }]
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 出站/自己发送的消息应被跳过
        assert!(msgs.is_empty(), "outbound/self messages should be skipped");
    }

    /// 测试跳过 is_from_me 标记的消息 - 旧版格式
    ///
    /// 验证对于旧版格式中 `is_from_me: true` 的消息，
    /// 解析器应返回空列表。
    #[test]
    fn linq_parse_skip_is_from_me() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造带有 is_from_me 标记的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "is_from_me": true,
                "message": {
                    "id": "msg-abc",
                    "parts": [{ "type": "text", "value": "My own message" }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // is_from_me 标记的消息应被跳过
        assert!(msgs.is_empty(), "is_from_me messages should be skipped");
    }

    /// 测试跳过非消息事件
    ///
    /// 验证对于 `event_type` 不是 `"message.received"` 的载荷，
    /// 解析器应返回空列表。例如消息投递确认等事件不应被处理。
    #[test]
    fn linq_parse_skip_non_message_event() {
        let ch = make_channel();
        // 构造非 message.received 事件类型的载荷
        let payload = serde_json::json!({
            "event_type": "message.delivered",
            "data": {
                "chat_id": "chat-789",
                "message_id": "msg-abc"
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 非消息事件应被跳过
        assert!(msgs.is_empty(), "Non-message events should be skipped");
    }

    /// 测试过滤未授权发送者
    ///
    /// 验证当发送者不在允许列表中时，
    /// 即使消息格式正确，解析器也应返回空列表。
    #[test]
    fn linq_parse_unauthorized_sender() {
        let ch = make_channel();
        // 构造来自未授权发送者的消息载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+9999999999",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [{ "type": "text", "value": "Spam" }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 未授权发送者的消息应被过滤
        assert!(msgs.is_empty(), "Unauthorized senders should be filtered");
    }

    /// 测试解析空载荷
    ///
    /// 验证当收到空 JSON 对象时，
    /// 解析器应安全地返回空列表而非崩溃。
    #[test]
    fn linq_parse_empty_payload() {
        let ch = make_channel();
        // 构造空 JSON 对象
        let payload = serde_json::json!({});
        let msgs = ch.parse_webhook_payload(&payload);
        // 空载荷应返回空列表
        assert!(msgs.is_empty());
    }

    /// 测试纯媒体消息转换为图片标记
    ///
    /// 验证当消息仅包含图片类型媒体时，
    /// 解析器应将其转换为 `[IMAGE:url]` 格式的文本标记。
    #[test]
    fn linq_parse_media_only_translated_to_image_marker() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造仅包含图片媒体的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [{
                        "type": "media",
                        "url": "https://example.com/image.jpg",
                        "mime_type": "image/jpeg"
                    }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 验证图片被转换为标记格式
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "[IMAGE:https://example.com/image.jpg]");
    }

    /// 测试非图片媒体被跳过
    ///
    /// 验证当消息仅包含非图片类型媒体（如音频）时，
    /// 解析器应返回空列表，因为当前不支持处理此类媒体。
    #[test]
    fn linq_parse_media_non_image_still_skipped() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造包含音频媒体的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [{
                        "type": "media",
                        "url": "https://example.com/sound.mp3",
                        "mime_type": "audio/mpeg"
                    }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 非图片媒体应被跳过
        assert!(msgs.is_empty(), "Non-image media should still be skipped");
    }

    /// 测试多部分文本消息合并
    ///
    /// 验证当消息包含多个文本部分时，
    /// 解析器应将它们用换行符连接成一个消息内容。
    #[test]
    fn linq_parse_multiple_text_parts() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造包含多个文本部分的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [
                        { "type": "text", "value": "First part" },
                        { "type": "text", "value": "Second part" }
                    ]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 验证多部分文本被正确合并
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "First part\nSecond part");
    }

    /// 测试专用的 Webhook 密钥常量
    ///
    /// 该常量仅用于签名验证相关的单元测试，
    /// 不是真实的凭证，不应用于生产环境。
    const TEST_WEBHOOK_SECRET: &str = "test_webhook_secret";

    /// 测试签名验证 - 有效签名
    ///
    /// 验证使用正确的密钥和时间戳生成的签名应通过验证。
    ///
    /// # 签名生成算法
    ///
    /// 1. 构造消息：`"{timestamp}.{body}"`
    /// 2. 使用 HMAC-SHA256 计算签名
    /// 3. 将签名编码为十六进制字符串
    #[test]
    fn linq_signature_verification_valid() {
        let secret = TEST_WEBHOOK_SECRET;
        let body = r#"{"event_type":"message.received"}"#;
        let now = chrono::Utc::now().timestamp().to_string();

        // 计算预期签名
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let message = format!("{now}.{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // 有效签名应通过验证
        assert!(verify_linq_signature(secret, body, &now, &signature));
    }

    /// 测试签名验证 - 无效签名
    ///
    /// 验证错误的签名应被拒绝。
    #[test]
    fn linq_signature_verification_invalid() {
        let secret = TEST_WEBHOOK_SECRET;
        let body = r#"{"event_type":"message.received"}"#;
        let now = chrono::Utc::now().timestamp().to_string();

        // 使用伪造的签名应验证失败
        assert!(!verify_linq_signature(secret, body, &now, "deadbeefdeadbeefdeadbeef"));
    }

    /// 测试签名验证 - 过期时间戳
    ///
    /// 验证即使签名正确，但时间戳超过 300 秒（5分钟）的请求应被拒绝。
    /// 这是防止重放攻击的安全措施。
    #[test]
    fn linq_signature_verification_stale_timestamp() {
        let secret = TEST_WEBHOOK_SECRET;
        let body = r#"{"event_type":"message.received"}"#;
        // 使用 10 分钟前的时间戳 - 已过期
        let stale_ts = (chrono::Utc::now().timestamp() - 600).to_string();

        // 即使签名正确，过期时间戳也应验证失败
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let message = format!("{stale_ts}.{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(
            !verify_linq_signature(secret, body, &stale_ts, &signature),
            "Stale timestamps (>300s) should be rejected"
        );
    }

    /// 测试签名验证 - 接受 sha256= 前缀格式
    ///
    /// 验证签名可以带有 `sha256=` 前缀，
    /// 解析器应正确处理这种格式。
    #[test]
    fn linq_signature_verification_accepts_sha256_prefix() {
        let secret = TEST_WEBHOOK_SECRET;
        let body = r#"{"event_type":"message.received"}"#;
        let now = chrono::Utc::now().timestamp().to_string();

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let message = format!("{now}.{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        // 添加 sha256= 前缀
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        // 带 sha256= 前缀的签名应通过验证
        assert!(verify_linq_signature(secret, body, &now, &signature));
    }

    /// 测试签名验证 - 接受大写十六进制格式
    ///
    /// 验证签名可以使用大写十六进制字符，
    /// 解析器应正确处理大小写不敏感的十六进制格式。
    #[test]
    fn linq_signature_verification_accepts_uppercase_hex() {
        let secret = TEST_WEBHOOK_SECRET;
        let body = r#"{"event_type":"message.received"}"#;
        let now = chrono::Utc::now().timestamp().to_string();

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let message = format!("{now}.{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(message.as_bytes());
        // 转换为大写十六进制
        let signature = hex::encode(mac.finalize().into_bytes()).to_ascii_uppercase();

        // 大写十六进制签名应通过验证
        assert!(verify_linq_signature(secret, body, &now, &signature));
    }

    /// 测试电话号码规范化 - 自动添加 + 前缀
    ///
    /// 验证当 API 发送不带 `+` 前缀的电话号码时，
    /// 解析器应自动规范化为带 `+` 前缀的格式，
    /// 以便与允许列表中的号码正确匹配。
    #[test]
    fn linq_parse_normalizes_phone_with_plus() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["+1234567890".into()]);
        // API 发送不带 + 前缀的号码，应被规范化为带 + 前缀
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [{ "type": "text", "value": "Hi" }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 验证号码被规范化且消息被正确解析
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    /// 测试解析缺少 data 字段的载荷
    ///
    /// 验证当载荷缺少 `data` 字段时，
    /// 解析器应安全地返回空列表。
    #[test]
    fn linq_parse_missing_data() {
        let ch = make_channel();
        // 构造缺少 data 字段的载荷
        let payload = serde_json::json!({
            "event_type": "message.received"
        });
        let msgs = ch.parse_webhook_payload(&payload);
        // 缺少 data 字段应返回空列表
        assert!(msgs.is_empty());
    }

    /// 测试解析缺少消息部分的载荷
    ///
    /// 验证当消息对象缺少 `parts` 字段时，
    /// 解析器应安全地返回空列表。
    #[test]
    fn linq_parse_missing_message_parts() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造缺少 parts 字段的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc"
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 缺少 parts 字段应返回空列表
        assert!(msgs.is_empty());
    }

    /// 测试解析空文本值
    ///
    /// 验证当文本部分的值为空字符串时，
    /// 解析器应跳过该消息并返回空列表。
    #[test]
    fn linq_parse_empty_text_value() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造包含空文本值的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "chat_id": "chat-789",
                "from": "+1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [{ "type": "text", "value": "" }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        // 空文本应被跳过
        assert!(msgs.is_empty(), "Empty text should be skipped");
    }

    /// 测试回复目标回退逻辑 - 无 chat_id 时使用发送者号码
    ///
    /// 验证当载荷中没有 `chat_id` 字段时，
    /// 解析器应将发送者电话号码作为回复目标。
    #[test]
    fn linq_parse_fallback_reply_target_when_no_chat_id() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        // 构造缺少 chat_id 的载荷
        let payload = serde_json::json!({
            "event_type": "message.received",
            "data": {
                "from": "+1234567890",
                "is_from_me": false,
                "message": {
                    "id": "msg-abc",
                    "parts": [{ "type": "text", "value": "Hi" }]
                }
            }
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        // 当没有 chat_id 时，回退到发送者电话号码
        assert_eq!(msgs[0].reply_target, "+1234567890");
    }

    /// 测试电话号码访问器
    ///
    /// 验证 `LinqChannel::phone_number()` 方法
    /// 返回正确的配置电话号码。
    #[test]
    fn linq_phone_number_accessor() {
        let ch = make_channel();
        assert_eq!(ch.phone_number(), "+15551234567");
    }

    #[test]
    fn linq_parse_uses_sender_and_media_value_fallbacks() {
        let ch = LinqChannel::new("tok".into(), "+15551234567".into(), vec!["*".into()]);
        let payload = serde_json::json!({
            "event_type": "message.received",
            "created_at": "invalid timestamp",
            "data": {
                "sender": "1234567890",
                "chat_id": "chat-789",
                "message": {
                    "parts": [{
                        "type": "image",
                        "value": " https://example.com/fallback.png ",
                        "mime_type": " IMAGE/PNG "
                    }]
                }
            }
        });

        let before = chrono::Utc::now().timestamp().cast_unsigned();
        let messages = ch.parse_webhook_payload(&payload);
        let after = chrono::Utc::now().timestamp().cast_unsigned();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, "+1234567890");
        assert_eq!(messages[0].content, "[IMAGE:https://example.com/fallback.png]");
        assert!((before..=after).contains(&messages[0].timestamp));
    }
}
