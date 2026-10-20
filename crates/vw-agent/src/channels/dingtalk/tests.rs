//! 钉钉通道单元测试模块
//!
//! 本模块包含 DingTalkChannel 及相关配置的单元测试用例，覆盖以下功能：
//! - 通道名称验证
//! - 用户访问权限控制（白名单机制）
//! - 配置序列化/反序列化
//! - 流数据解析
//! - 聊天ID解析
//!
//! 所有测试遵循隔离原则，不依赖外部钉钉服务。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];

        loop {
            let n = stream.read(&mut tmp).await.unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);

            let Some(header_end) = buf.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&buf[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);

            if buf.len() >= header_end + 4 + content_length {
                break;
            }
        }

        String::from_utf8_lossy(&buf).into_owned()
    }

    async fn one_shot_http_server(
        status_line: &'static str,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            let response = format!(
                "{status_line}\r\ncontent-length: {}\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            request
        });

        (format!("http://{addr}/webhook"), handle)
    }

    fn request_body(request: &str) -> serde_json::Value {
        let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
        serde_json::from_str(body).unwrap()
    }

    /// 测试通道名称返回值
    ///
    /// 验证 DingTalkChannel 实例的 `name()` 方法正确返回 "dingtalk" 标识符。
    #[test]
    fn test_name() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec![]);
        assert_eq!(ch.name(), "dingtalk");
    }

    /// 测试通配符用户访问权限
    ///
    /// 当允许用户列表包含 "*" 通配符时，任意用户都应被允许访问。
    /// 这是白名单的"全开放"模式。
    #[test]
    fn test_user_allowed_wildcard() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        assert!(ch.is_user_allowed("anyone"));
    }

    /// 测试特定用户访问权限
    ///
    /// 当允许用户列表包含特定用户ID时：
    /// - 列表中的用户应被允许访问
    /// - 不在列表中的用户应被拒绝
    #[test]
    fn test_user_allowed_specific() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["user123".into()]);
        assert!(ch.is_user_allowed("user123"));
        assert!(!ch.is_user_allowed("other"));
    }

    /// 测试空允许列表的访问权限
    ///
    /// 当允许用户列表为空时，所有用户都应被拒绝访问。
    /// 这是白名单的"全关闭"模式，用于默认安全策略。
    #[test]
    fn test_user_denied_empty() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec![]);
        assert!(!ch.is_user_allowed("anyone"));
    }

    /// 测试配置 TOML 反序列化
    ///
    /// 验证 DingTalkConfig 能正确从 TOML 字符串解析，包括：
    /// - client_id: 钉钉应用ID
    /// - client_secret: 钉钉应用密钥
    /// - allowed_users: 允许的用户列表
    #[test]
    fn test_config_serde() {
        let toml_str = r#"
    client_id = "app_id_123"
    client_secret = "secret_456"
    allowed_users = ["user1", "*"]
    "#;
        let config: crate::app::agent::config::schema::DingTalkConfig =
            toml::from_str(toml_str).unwrap();
        assert_eq!(config.client_id, "app_id_123");
        assert_eq!(config.client_secret, "secret_456");
        assert_eq!(config.allowed_users, vec!["user1", "*"]);
    }

    /// 测试配置 TOML 反序列化默认值
    ///
    /// 当 TOML 中未指定 allowed_users 字段时，
    /// 应使用默认值（空列表）。
    #[test]
    fn test_config_serde_defaults() {
        let toml_str = r#"
    client_id = "id"
    client_secret = "secret"
    "#;
        let config: crate::app::agent::config::schema::DingTalkConfig =
            toml::from_str(toml_str).unwrap();
        assert!(config.allowed_users.is_empty());
    }

    /// 测试流数据解析支持字符串载荷
    ///
    /// 钉钉回调可能将载荷作为转义的 JSON 字符串传递。
    /// 此测试验证 `parse_stream_data` 能正确解析字符串形式的载荷，
    /// 提取出嵌套的 text.content 字段。
    #[test]
    fn parse_stream_data_supports_string_payload() {
        let frame = serde_json::json!({
            "data": "{\"text\":{\"content\":\"hello\"}}"
        });
        let parsed = DingTalkChannel::parse_stream_data(&frame).unwrap();
        assert_eq!(
            parsed.get("text").and_then(|v| v.get("content")),
            Some(&serde_json::json!("hello"))
        );
    }

    /// 测试流数据解析支持对象载荷
    ///
    /// 钉钉回调也可能直接传递 JSON 对象作为载荷。
    /// 此测试验证 `parse_stream_data` 能正确处理对象形式的载荷，
    /// 提取出嵌套的 text.content 字段。
    #[test]
    fn parse_stream_data_supports_object_payload() {
        let frame = serde_json::json!({
            "data": {"text": {"content": "hello"}}
        });
        let parsed = DingTalkChannel::parse_stream_data(&frame).unwrap();
        assert_eq!(
            parsed.get("text").and_then(|v| v.get("content")),
            Some(&serde_json::json!("hello"))
        );
    }

    /// 测试聊天ID解析处理群聊类型
    ///
    /// 当 conversationType 为 2（群聊）时，应使用 conversationId 作为聊天ID，
    /// 而非发送者的 staffId。此测试验证群聊场景下的ID解析逻辑。
    #[test]
    fn resolve_chat_id_handles_numeric_group_conversation_type() {
        let data = serde_json::json!({
            "conversationType": 2,
            "conversationId": "cid-group",
        });
        let chat_id = DingTalkChannel::resolve_chat_id(&data, "staff-1");
        assert_eq!(chat_id, "cid-group");
    }

    #[test]
    fn parse_stream_data_rejects_missing_invalid_or_non_object_payloads() {
        assert!(DingTalkChannel::parse_stream_data(&serde_json::json!({})).is_none());
        assert!(
            DingTalkChannel::parse_stream_data(&serde_json::json!({"data": "not-json"})).is_none()
        );
        assert!(DingTalkChannel::parse_stream_data(&serde_json::json!({"data": 42})).is_none());
    }

    #[test]
    fn resolve_chat_id_handles_private_string_and_group_fallback() {
        let private = serde_json::json!({
            "conversationType": "1",
            "conversationId": "cid-private",
        });
        assert_eq!(DingTalkChannel::resolve_chat_id(&private, "staff-1"), "staff-1");

        let group_without_id = serde_json::json!({
            "conversationType": "2",
        });
        assert_eq!(DingTalkChannel::resolve_chat_id(&group_without_id, "staff-2"), "staff-2");
    }

    #[test]
    fn resolve_chat_id_defaults_missing_conversation_type_to_private() {
        let data = serde_json::json!({
            "conversationId": "group-looking-id",
        });

        assert_eq!(DingTalkChannel::resolve_chat_id(&data, "staff-1"), "staff-1");
    }

    #[tokio::test]
    async fn send_without_cached_webhook_fails_before_network() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let error = Channel::send(
            &ch,
            &SendMessage {
                content: "hello".into(),
                recipient: "missing-chat".into(),
                subject: Some("Title".into()),
                thread_ts: None,
            },
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("missing-chat"));
        assert!(error.contains("webhook"));
    }

    #[test]
    fn gateway_registration_body_contains_credentials_and_subscription() {
        let ch =
            DingTalkChannel::new("client-id".into(), "client-secret".into(), vec!["user-1".into()]);
        let body = ch.gateway_registration_body();

        assert_eq!(body["clientId"], "client-id");
        assert_eq!(body["clientSecret"], "client-secret");
        assert_eq!(body["subscriptions"][0]["type"], "CALLBACK");
        assert_eq!(body["subscriptions"][0]["topic"], DINGTALK_BOT_CALLBACK_TOPIC);
    }

    #[test]
    fn websocket_url_appends_ticket_query() {
        let gw = GatewayResponse {
            endpoint: "wss://example.test/connect".into(),
            ticket: "ticket-1".into(),
        };

        assert_eq!(
            DingTalkChannel::websocket_url(&gw),
            "wss://example.test/connect?ticket=ticket-1"
        );
    }

    #[tokio::test]
    async fn register_connection_posts_body_and_decodes_gateway_response() {
        let (url, server) = one_shot_http_server(
            "HTTP/1.1 200 OK",
            r#"{"endpoint":"wss://gateway.example.test/connect","ticket":"ticket-1"}"#,
        )
        .await;
        let ch = DingTalkChannel::new(
            "client-id".into(),
            "client-secret".into(),
            vec!["staff-1".into()],
        )
        .with_gateway_endpoint_for_test(url);

        let gw = ch.register_connection().await.expect("gateway registration");

        assert_eq!(gw.endpoint, "wss://gateway.example.test/connect");
        assert_eq!(gw.ticket, "ticket-1");
        let request = server.await.unwrap();
        assert!(request.starts_with("POST /webhook "));
        let body = request_body(&request);
        assert_eq!(body["clientId"], "client-id");
        assert_eq!(body["clientSecret"], "client-secret");
        assert_eq!(body["subscriptions"][0]["topic"], DINGTALK_BOT_CALLBACK_TOPIC);
    }

    #[tokio::test]
    async fn register_connection_reports_sanitized_gateway_error() {
        let (url, server) = one_shot_http_server(
            "HTTP/1.1 401 Unauthorized",
            r#"{"message":"client_secret sk-secret should be hidden"}"#,
        )
        .await;
        let ch = DingTalkChannel::new("client-id".into(), "client-secret".into(), vec![])
            .with_gateway_endpoint_for_test(url);

        let error = match ch.register_connection().await {
            Ok(_) => panic!("gateway registration should fail"),
            Err(error) => error.to_string(),
        };

        assert!(error.contains("钉钉网关注册失败"));
        assert!(error.contains("401 Unauthorized"));
        assert!(!error.contains("sk-secret"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn health_check_returns_false_when_gateway_registration_fails() {
        let (url, server) =
            one_shot_http_server("HTTP/1.1 500 Internal Server Error", r#"{}"#).await;
        let ch = DingTalkChannel::new("client-id".into(), "client-secret".into(), vec![])
            .with_gateway_endpoint_for_test(url);

        assert!(!ch.health_check().await);
        server.await.unwrap();
    }

    #[test]
    fn stream_response_frame_uses_message_id_and_ok_shape() {
        let frame = DingTalkChannel::stream_response_frame("msg-1");

        assert_eq!(frame["code"], 200);
        assert_eq!(frame["headers"]["contentType"], "application/json");
        assert_eq!(frame["headers"]["messageId"], "msg-1");
        assert_eq!(frame["message"], "OK");
        assert_eq!(frame["data"], "");
    }

    #[test]
    fn frame_message_id_returns_empty_for_missing_header() {
        let frame = serde_json::json!({});

        assert_eq!(DingTalkChannel::frame_message_id(&frame), "");
    }

    #[test]
    fn webhook_reply_body_uses_subject_or_default_title() {
        let with_subject =
            DingTalkChannel::webhook_reply_body(&SendMessage::with_subject("hello", "chat-1", "T"));
        let without_subject =
            DingTalkChannel::webhook_reply_body(&SendMessage::new("hello", "chat-1"));

        assert_eq!(with_subject["msgtype"], "markdown");
        assert_eq!(with_subject["markdown"]["title"], "T");
        assert_eq!(with_subject["markdown"]["text"], "hello");
        assert_eq!(without_subject["markdown"]["title"], "VibeWindow");
    }

    #[test]
    fn webhook_reply_body_normalizes_dingtalk_markdown() {
        let message = SendMessage::new(
            "说明\n1. 第一步\n2. 第二步\n\n    ```rust\n    fn main() {}\n    ```",
            "chat-1",
        );

        let body = DingTalkChannel::webhook_reply_body(&message);

        assert_eq!(
            body["markdown"]["text"],
            "说明\n\n1. 第一步\n2. 第二步\n\n```rust\nfn main() {}\n```"
        );
    }

    #[test]
    fn webhook_reply_body_uses_plain_text_for_long_content() {
        let content = "界".repeat(DINGTALK_MARKDOWN_MAX_CHARS + 1);
        let body = DingTalkChannel::webhook_reply_body(&SendMessage::new(&content, "chat-1"));

        assert_eq!(body["msgtype"], "text");
        assert_eq!(body["text"]["content"], content);
        assert!(body.get("markdown").is_none());
    }

    #[test]
    fn channel_message_from_data_builds_group_message() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["staff-1".into()]);
        let data = serde_json::json!({
            "text": {"content": "  hello group  "},
            "senderStaffId": "staff-1",
            "conversationType": 2,
            "conversationId": "group-1",
        });

        let msg = ch.channel_message_from_data(&data).unwrap();

        uuid::Uuid::parse_str(&msg.id).unwrap();
        assert_eq!(msg.sender, "staff-1");
        assert_eq!(msg.reply_target, "group-1");
        assert_eq!(msg.content, "hello group");
        assert_eq!(msg.channel, "dingtalk");
        assert!(msg.timestamp > 0);
        assert!(msg.thread_ts.is_none());
    }

    #[test]
    fn channel_message_from_data_skips_blank_or_denied_message() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["staff-1".into()]);
        let blank = serde_json::json!({
            "text": {"content": "   "},
            "senderStaffId": "staff-1",
        });
        let denied = serde_json::json!({
            "text": {"content": "hello"},
            "senderStaffId": "staff-2",
        });

        assert!(ch.channel_message_from_data(&blank).is_none());
        assert!(ch.channel_message_from_data(&denied).is_none());
    }

    #[tokio::test]
    async fn cache_session_webhook_stores_chat_and_sender_routes() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        let data = serde_json::json!({
            "sessionWebhook": "https://webhook.example.test/reply"
        });

        ch.cache_session_webhook("chat-1", "staff-1", &data).await;

        let webhooks = ch.session_webhooks.read().await;
        assert_eq!(
            webhooks.get("chat-1").map(String::as_str),
            Some("https://webhook.example.test/reply")
        );
        assert_eq!(
            webhooks.get("staff-1").map(String::as_str),
            Some("https://webhook.example.test/reply")
        );
    }

    #[tokio::test]
    async fn cache_session_webhook_ignores_missing_webhook() {
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);

        ch.cache_session_webhook("chat-1", "staff-1", &serde_json::json!({})).await;

        assert!(ch.session_webhooks.read().await.is_empty());
    }

    #[tokio::test]
    async fn send_posts_cached_webhook_markdown_body() {
        let (url, server) = one_shot_http_server("HTTP/1.1 200 OK", "{}").await;
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        ch.session_webhooks.write().await.insert("chat-1".into(), url);

        Channel::send(&ch, &SendMessage::with_subject("hello", "chat-1", "Title")).await.unwrap();

        let request = server.await.unwrap();
        assert!(request.starts_with("POST /webhook "));
        let body = request_body(&request);
        assert_eq!(body["msgtype"], "markdown");
        assert_eq!(body["markdown"]["title"], "Title");
        assert_eq!(body["markdown"]["text"], "hello");
    }

    #[tokio::test]
    async fn send_reports_sanitized_webhook_error() {
        let (url, server) = one_shot_http_server(
            "HTTP/1.1 500 Internal Server Error",
            r#"{"message":"token sk-secret should be hidden"}"#,
        )
        .await;
        let ch = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        ch.session_webhooks.write().await.insert("chat-1".into(), url);

        let error =
            Channel::send(&ch, &SendMessage::new("hello", "chat-1")).await.unwrap_err().to_string();

        assert!(error.contains("钉钉 webhook 回复失败"));
        assert!(error.contains("500 Internal Server Error"));
        assert!(!error.contains("sk-secret"));
        server.await.unwrap();
    }
}
