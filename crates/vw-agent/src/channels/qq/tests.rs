//! QQ 频道模块单元测试
//!
//! 本模块包含 QQ 频道（`QQChannel`）及其相关辅助函数的全面测试套件。
//! 测试覆盖以下关键功能：
//!
//! - **基础属性**：验证频道名称、配置序列化/反序列化
//! - **权限控制**：测试用户访问控制列表（允许列表）的通配符和精确匹配逻辑
//! - **消息去重**：验证基于消息 ID 的重复消息检测机制
//! - **Webhook 处理**：测试 Webhook 验证响应构建、消息解析、去重逻辑
//! - **消息内容处理**：测试消息内容组合（文本+附件）、出站内容解析、
//!   消息体构建（文本/媒体消息）
//!
//! 所有测试均为隔离的单元测试，不依赖外部 QQ API 或网络连接。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use serde_json::json;

    /// 测试 `QQChannel::name()` 方法返回正确的频道标识符。
    ///
    /// 验证创建的 QQ 频道实例的 `name()` 方法返回字符串 `"qq"`，
    /// 这是该 Channel trait 实现的固定标识符，用于在系统中区分不同的通道类型。
    #[test]
    fn test_name() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec![]);
        assert_eq!(ch.name(), "qq");
    }

    /// 测试用户允许列表的通配符匹配逻辑。
    ///
    /// 当 `allowed_users` 包含通配符 `"*"` 时，`is_user_allowed()` 方法应对任意
    /// 用户名返回 `true`，实现完全开放的访问策略。
    ///
    /// # 测试场景
    /// - 允许列表：`["*"]`
    /// - 测试用户：`"anyone"`
    /// - 预期结果：允许访问
    #[test]
    fn test_user_allowed_wildcard() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("anyone"));
    }

    /// 测试用户允许列表的精确匹配逻辑。
    ///
    /// 当 `allowed_users` 包含特定用户 ID 时，`is_user_allowed()` 方法应：
    /// - 对匹配的用户 ID 返回 `true`
    /// - 对不匹配的用户 ID 返回 `false`
    ///
    /// # 测试场景
    /// - 允许列表：`["user123"]`
    /// - 测试用户 1：`"user123"` → 预期允许
    /// - 测试用户 2：`"other"` → 预期拒绝
    #[test]
    fn test_user_allowed_specific() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec!["user123".into()]);
        assert!(ch.is_user_allowed("user123"));
        assert!(!ch.is_user_allowed("other"));
    }

    /// 测试空允许列表的默认拒绝策略。
    ///
    /// 当 `allowed_users` 为空列表时，`is_user_allowed()` 方法应对所有用户
    /// 返回 `false`，实现默认拒绝（deny-by-default）的安全策略。
    ///
    /// # 测试场景
    /// - 允许列表：`[]`（空）
    /// - 测试用户：`"anyone"`
    /// - 预期结果：拒绝访问
    #[test]
    fn test_user_denied_empty() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec![]);
        assert!(!ch.is_user_allowed("anyone"));
    }

    /// 测试基于消息 ID 的去重机制。
    ///
    /// `is_duplicate()` 方法使用内部 LRU 缓存跟踪最近处理的消息 ID，
    /// 确保相同的消息不会被重复处理。
    ///
    /// # 测试场景
    /// 1. 首次检查消息 ID `"msg1"` → 返回 `false`（非重复）
    /// 2. 再次检查消息 ID `"msg1"` → 返回 `true`（重复）
    /// 3. 首次检查消息 ID `"msg2"` → 返回 `false`（非重复）
    ///
    /// 这验证了去重状态的持久性和独立性。
    #[tokio::test]
    async fn test_dedup() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec![]);
        assert!(!ch.is_duplicate("msg1").await);
        assert!(ch.is_duplicate("msg1").await);
        assert!(!ch.is_duplicate("msg2").await);
    }

    /// 测试空消息 ID 永远不被视为重复。
    ///
    /// 空字符串消息 ID 通常是无效或占位符，不应被去重逻辑拦截。
    /// 即使连续检查空 ID，也应始终返回 `false`。
    ///
    /// # 测试场景
    /// - 连续两次检查空字符串 `""`
    /// - 预期结果：均返回 `false`（非重复）
    ///
    /// 这确保了边界情况的健壮性。
    #[tokio::test]
    async fn test_dedup_empty_id() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec![]);
        // 空消息 ID 永远不应被视为重复消息
        assert!(!ch.is_duplicate("").await);
        assert!(!ch.is_duplicate("").await);
    }

    /// 测试去重集合达到容量时会收缩后再记录新消息。
    #[tokio::test]
    async fn dedup_at_capacity_prunes_half_before_recording_new_message() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec![]);
        {
            let mut dedup = ch.dedup.write().await;
            dedup.extend((0..DEDUP_CAPACITY).map(|index| format!("msg-{index}")));
        }

        assert!(!ch.is_duplicate("overflow-message").await);

        let dedup = ch.dedup.read().await;
        assert_eq!(dedup.len(), DEDUP_CAPACITY / 2 + 1);
        assert!(dedup.contains("overflow-message"));
    }

    /// 测试 `QQConfig` 的 TOML 反序列化。
    ///
    /// 验证配置结构能正确解析 TOML 格式的配置字符串，
    /// 包括必需字段和默认值。
    ///
    /// # 测试场景
    /// - 解析包含 `app_id`、`app_secret`、`allowed_users` 的 TOML
    /// - 验证字段值正确映射
    /// - 验证 `receive_mode` 使用默认值 `Webhook`
    #[test]
    fn test_config_serde() {
        let toml_str = r#"
    app_id = "12345"
    app_secret = "secret_abc"
    allowed_users = ["user1"]
    "#;
        let config: crate::app::agent::config::schema::QQConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.app_id, "12345");
        assert_eq!(config.app_secret, "secret_abc");
        assert_eq!(config.allowed_users, vec!["user1"]);
        assert_eq!(config.receive_mode, crate::app::agent::config::schema::QQReceiveMode::Webhook);
    }

    /// 测试 Webhook 验证响应的构建。
    ///
    /// 当 QQ 平台发送 Webhook 验证请求（`op=13`）时，需要计算 HMAC-SHA256
    /// 签名并返回包含 `plain_token` 和 `signature` 的响应。
    ///
    /// # 测试场景
    /// - 使用固定的 `app_id` 和 `app_secret`
    /// - 输入包含 `op=13`、`plain_token` 和 `event_ts` 的 payload
    /// - 验证返回的响应包含正确的 `plain_token` 和预期的签名
    ///
    /// # 签名计算
    /// 签名 = HMAC-SHA256(app_secret, plain_token + event_ts)
    #[test]
    fn test_build_webhook_validation_response() {
        let ch = QQChannel::new("11111111".into(), "DG5g3B4j9X2KOErG".into(), vec!["*".into()]);
        let payload = json!({
            "op": 13,
            "d": {
                "plain_token": "Arq0D5A61EgUu4OxUvOp",
                "event_ts": "1725442341"
            }
        });

        let response =
            ch.build_webhook_validation_response(&payload).expect("validation response expected");

        assert_eq!(response["plain_token"], "Arq0D5A61EgUu4OxUvOp");
        assert_eq!(
            response["signature"],
            "87befc99c42c651b3aac0278e71ada338433ae26fcb24307bdc5ad38c1adc2d01bcfcadc0842edac85e85205028a1132afe09280305f13aa6909ffc2d652c706"
        );
    }

    /// 测试解析 C2C（私聊）Webhook 消息事件。
    ///
    /// 验证 `parse_webhook_payload()` 能正确解析私聊消息事件（`C2C_MESSAGE_CREATE`），
    /// 提取发送者信息、回复目标和消息线程标识。
    ///
    /// # 测试场景
    /// - 输入：`op=0`（事件）、`t=C2C_MESSAGE_CREATE`（私聊消息创建事件）
    /// - 消息包含：`id`、`content`、`author.id`、`author.user_openid`
    /// - 验证输出 `IncomingMessage` 结构：
    ///   - `sender`: 提取为 `user_openid`
    ///   - `reply_target`: 格式为 `user:{openid}`
    ///   - `thread_ts`: 使用消息 ID
    #[tokio::test]
    async fn test_parse_webhook_payload_c2c_event() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec!["user_open_1".into()]);
        let payload = json!({
            "op": 0,
            "t": "C2C_MESSAGE_CREATE",
            "d": {
                "id": "msg-1",
                "content": "hello webhook",
                "author": {
                    "id": "author-1",
                    "user_openid": "user_open_1"
                }
            }
        });

        let messages = ch.parse_webhook_payload(&payload).await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, "user_open_1");
        assert_eq!(messages[0].reply_target, "user:user_open_1");
        assert_eq!(messages[0].thread_ts.as_deref(), Some("msg-1"));
    }

    /// 测试 Webhook 消息解析的去重功能。
    ///
    /// 验证 `parse_webhook_payload()` 在解析消息时会自动检查去重状态，
    /// 相同消息 ID 的重复请求不会产生重复的 `IncomingMessage`。
    ///
    /// # 测试场景
    /// 1. 首次解析消息 ID 为 `"msg-dup"` 的 payload → 返回 1 条消息
    /// 2. 再次解析相同 payload → 返回 0 条消息（被去重）
    ///
    /// 这确保了消息处理的幂等性。
    #[tokio::test]
    async fn test_parse_webhook_payload_deduplicates_by_message_id() {
        let ch = QQChannel::new("id".into(), "secret".into(), vec!["user_open_1".into()]);
        let payload = json!({
            "op": 0,
            "t": "C2C_MESSAGE_CREATE",
            "d": {
                "id": "msg-dup",
                "content": "hello webhook",
                "author": {
                    "id": "author-1",
                    "user_openid": "user_open_1"
                }
            }
        });

        let first = ch.parse_webhook_payload(&payload).await;
        let second = ch.parse_webhook_payload(&payload).await;
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    /// 测试 `compose_message_content()` 处理纯文本消息。
    ///
    /// 当消息仅包含文本内容时，应去除首尾空白并返回清理后的文本。
    ///
    /// # 测试场景
    /// - 输入 payload：`content = "  hello world  "`（带前后空格）
    /// - 预期输出：`Some("hello world")`（已修剪）
    #[test]
    fn test_compose_message_content_text_only() {
        let payload = json!({
            "content": "  hello world  "
        });

        assert_eq!(compose_message_content(&payload), Some("hello world".to_string()));
    }

    /// 测试 `compose_message_content()` 处理仅有图片附件的消息。
    ///
    /// 当消息文本为空或仅含空白字符，但包含图片附件时，
    /// 应生成 `[IMAGE:url]` 格式的占位符。
    ///
    /// # 测试场景
    /// - `content`: 空白字符串 `"   "`
    /// - `attachments`: 包含一个 `image/jpg` 类型的附件
    /// - 预期输出：`Some("[IMAGE:https://cdn.example.com/a.jpg]")`
    #[test]
    fn test_compose_message_content_attachment_only_image() {
        let payload = json!({
            "content": "   ",
            "attachments": [
                {
                    "content_type": "image/jpg",
                    "url": "https://cdn.example.com/a.jpg"
                }
            ]
        });

        assert_eq!(
            compose_message_content(&payload),
            Some("[IMAGE:https://cdn.example.com/a.jpg]".to_string())
        );
    }

    /// 测试 `compose_message_content()` 处理文本与多图片附件的组合。
    ///
    /// 当消息同时包含文本内容和多个图片附件时，应将它们组合：
    /// - 文本在前
    /// - 图片占位符在后，每个占位符独立一行
    ///
    /// # 测试场景
    /// - `content`: `"Here is an image"`
    /// - `attachments`: 两个图片（一个通过 `content_type` 识别，一个通过 `filename` 识别）
    /// - 预期输出：文本 + 空行 + 两个 `[IMAGE:url]` 标记
    #[test]
    fn test_compose_message_content_text_and_image_attachments() {
        let payload = json!({
            "content": "Here is an image",
            "attachments": [
                {
                    "content_type": "image/png",
                    "url": "https://cdn.example.com/a.png"
                },
                {
                    "filename": "b.jpeg",
                    "url": "https://cdn.example.com/b.jpeg"
                }
            ]
        });

        assert_eq!(
                compose_message_content(&payload),
                Some(
                    "Here is an image\n\n[IMAGE:https://cdn.example.com/a.png]\n[IMAGE:https://cdn.example.com/b.jpeg]"
                        .to_string()
                )
            );
    }

    /// 测试 `compose_message_content()` 忽略非图片类型的附件。
    ///
    /// 只有图片类型的附件（通过 `content_type` 或 `filename` 识别）
    /// 会被转换为 `[IMAGE:url]` 标记，其他类型（如 PDF）应被忽略。
    ///
    /// # 测试场景
    /// - `content`: `"text"`
    /// - `attachments`: 一个 `application/pdf` 类型的附件
    /// - 预期输出：仅保留文本，忽略 PDF 附件
    #[test]
    fn test_compose_message_content_ignores_non_image_attachments() {
        let payload = json!({
            "content": "text",
            "attachments": [
                {
                    "content_type": "application/pdf",
                    "url": "https://cdn.example.com/a.pdf"
                }
            ]
        });

        assert_eq!(compose_message_content(&payload), Some("text".to_string()));
    }

    /// 测试 `compose_message_content()` 对无效消息返回 `None`。
    ///
    /// 当消息既没有有效文本内容，也没有有效的图片附件时，
    /// 应返回 `None` 表示无法提取任何有意义的消息内容。
    ///
    /// # 测试场景
    /// - `content`: 空白字符串
    /// - `attachments`:
    ///   - 一个非图片附件（PDF）
    ///   - 一个图片附件但 URL 为空白
    /// - 预期输出：`None`（无有效内容）
    #[test]
    fn test_compose_message_content_drops_empty_without_valid_attachments() {
        let payload = json!({
            "content": "   ",
            "attachments": [
                {
                    "content_type": "application/pdf",
                    "url": "https://cdn.example.com/a.pdf"
                },
                {
                    "content_type": "image/png",
                    "url": "   "
                }
            ]
        });

        assert_eq!(compose_message_content(&payload), None);
    }

    /// 测试 `parse_outgoing_content()` 提取远程图片 URL 标记。
    ///
    /// 出站消息中的 `[IMAGE:url]` 标记需要被解析和提取，
    /// 以便将图片作为媒体消息单独发送。
    ///
    /// # 测试场景
    /// - 输入文本包含两个 `[IMAGE:url]` 标记（一个 HTTPS，一个 HTTP）
    /// - 预期输出：
    ///   - `text`: 去除标记后的纯文本
    ///   - `images`: 提取的远程 URL 列表
    ///
    /// # 注意
    /// 只有以 `http://` 或 `https://` 开头的 URL 才会被提取，
    /// 本地路径（如 `/tmp/a.png`）会被保留在文本中。
    #[test]
    fn test_parse_outgoing_content_extracts_remote_image_markers() {
        let input = "hello\n[IMAGE:https://cdn.example.com/a.png]\n[IMAGE:http://cdn.example.com/b.jpg]\nbye";
        let (text, images) = parse_outgoing_content(input);

        assert_eq!(text, "hello\nbye");
        assert_eq!(
            images,
            vec![
                "https://cdn.example.com/a.png".to_string(),
                "http://cdn.example.com/b.jpg".to_string()
            ]
        );
    }

    /// 测试 `parse_outgoing_content()` 保留非远程图片标记。
    ///
    /// 当 `[IMAGE:...]` 标记中的 URL 不是远程地址（不以 http/https 开头）时，
    /// 该标记应被视为普通文本保留，不进行提取。
    ///
    /// # 测试场景
    /// - 输入：`"[IMAGE:/tmp/a.png]\nhello"`（本地路径）
    /// - 预期输出：
    ///   - `text`: 原样保留标记和文本
    ///   - `images`: 空列表（无提取）
    #[test]
    fn test_parse_outgoing_content_keeps_non_remote_image_marker_as_text() {
        let input = "[IMAGE:/tmp/a.png]\nhello";
        let (text, images) = parse_outgoing_content(input);

        assert_eq!(text, "[IMAGE:/tmp/a.png]\nhello");
        assert!(images.is_empty());
    }

    /// 测试 `build_text_message_body()` 构建被动回复的文本消息体。
    ///
    /// 当回复私聊消息时，需要包含被动消息字段（`msg_id` 和 `msg_seq`）
    /// 以符合 QQ API 的消息引用要求。
    ///
    /// # 测试场景
    /// - 输入：文本内容 `"hello"`、引用消息 ID `"msg-123"`、序列号 `2`
    /// - 预期输出 JSON：
    ///   - `content`: `"hello"`
    ///   - `msg_type`: `0`（文本消息）
    ///   - `msg_id`: `"msg-123"`（被动回复引用）
    ///   - `msg_seq`: `2`（消息序列号）
    #[test]
    fn test_build_text_message_body_with_passive_fields() {
        let body = build_text_message_body("hello", Some("msg-123"), 2).expect("text body");
        assert_eq!(
            body,
            json!({
                "content": "hello",
                "msg_type": 0,
                "msg_id": "msg-123",
                "msg_seq": 2
            })
        );
    }

    /// 测试 `build_media_message_body()` 构建被动回复的媒体消息体。
    ///
    /// 当发送媒体（图片）消息作为被动回复时，需要包含被动消息字段
    /// 以及媒体文件信息。
    ///
    /// # 测试场景
    /// - 输入：文件信息 `"file-info-abc"`、引用消息 ID `"msg-123"`、序列号 `3`
    /// - 预期输出 JSON：
    ///   - `content`: `" "`（占位空格）
    ///   - `msg_type`: `7`（媒体消息）
    ///   - `media.file_info`: `"file-info-abc"`
    ///   - `msg_id`: `"msg-123"`（被动回复引用）
    ///   - `msg_seq`: `3`（消息序列号）
    #[test]
    fn test_build_media_message_body_with_passive_fields() {
        let body = build_media_message_body("file-info-abc", Some("msg-123"), 3);
        assert_eq!(
            body,
            json!({
                "content": " ",
                "msg_type": 7,
                "media": {
                    "file_info": "file-info-abc"
                },
                "msg_id": "msg-123",
                "msg_seq": 3
            })
        );
    }
}
