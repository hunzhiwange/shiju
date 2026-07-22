//! Signal 通道测试模块
//!
//! 本模块包含 SignalChannel 及其相关数据结构的单元测试。
//! 测试覆盖以下功能领域：
//! - 通道创建与字段验证
//! - 发送者权限检查（允许列表、通配符）
//! - 群组消息匹配逻辑
//! - 回复目标解析与路由
//! - UUID 格式验证
//! - 消息信封处理（直接消息、群组消息、隐私用户）
//! - SSE 事件反序列化
//!
//! 这些测试确保 Signal 通道的实现符合设计规范，
//! 包括安全边界（权限过滤）和消息路由正确性。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::channels::traits::SendMessage;
    use axum::{
        Router,
        body::Bytes,
        extract::{OriginalUri, State},
        http::{Method, StatusCode},
    };
    use std::{collections::VecDeque, sync::Arc};
    use tokio::sync::{Mutex, oneshot};

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: Method,
        path: String,
        body: serde_json::Value,
    }

    #[derive(Clone, Debug)]
    struct ResponseSpec {
        status: StatusCode,
        body: &'static str,
    }

    impl ResponseSpec {
        fn ok(body: &'static str) -> Self {
            Self { status: StatusCode::OK, body }
        }

        fn created() -> Self {
            Self { status: StatusCode::CREATED, body: "" }
        }
    }

    struct TestServerState {
        requests: Mutex<Vec<RecordedRequest>>,
        responses: Mutex<VecDeque<ResponseSpec>>,
    }

    struct TestServer {
        base_url: String,
        state: Arc<TestServerState>,
        shutdown: Option<oneshot::Sender<()>>,
    }

    impl TestServer {
        async fn spawn(responses: Vec<ResponseSpec>) -> Self {
            let state = Arc::new(TestServerState {
                requests: Mutex::new(Vec::new()),
                responses: Mutex::new(VecDeque::from(responses)),
            });
            let app = Router::new().fallback(record_request).with_state(state.clone());
            let listener =
                tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind test server");
            let addr = listener.local_addr().expect("server addr");
            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            tokio::spawn(async move {
                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = shutdown_rx.await;
                    })
                    .await
                    .expect("serve test server");
            });

            Self { base_url: format!("http://{addr}"), state, shutdown: Some(shutdown_tx) }
        }

        async fn requests(&self) -> Vec<RecordedRequest> {
            self.state.requests.lock().await.clone()
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            if let Some(shutdown) = self.shutdown.take() {
                let _ = shutdown.send(());
            }
        }
    }

    async fn record_request(
        State(state): State<Arc<TestServerState>>,
        method: Method,
        uri: OriginalUri,
        body: Bytes,
    ) -> (StatusCode, String) {
        let parsed_body = if body.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&body).unwrap_or_else(|_| {
                serde_json::Value::String(String::from_utf8_lossy(&body).into_owned())
            })
        };
        state.requests.lock().await.push(RecordedRequest {
            method,
            path: uri.path().to_string(),
            body: parsed_body,
        });

        let response = state
            .responses
            .lock()
            .await
            .pop_front()
            .unwrap_or_else(|| ResponseSpec::ok(r#"{"result":true}"#));
        (response.status, response.body.to_string())
    }

    /// 创建一个标准的 SignalChannel 实例用于测试
    ///
    /// 配置参数：
    /// - HTTP URL: http://127.0.0.1:8686
    /// - 账号: +1234567890
    /// - 无群组 ID（用于直接消息）
    /// - 允许列表: ["+1111111111"]
    /// - 不忽略附件
    /// - 不忽略故事消息
    ///
    /// # 返回值
    /// 配置好的 SignalChannel 实例
    fn make_channel() -> SignalChannel {
        SignalChannel::new(
            "http://127.0.0.1:8686".to_string(),
            "+1234567890".to_string(),
            None,
            vec!["+1111111111".to_string()],
            false,
            false,
        )
    }

    /// 创建一个配置了群组 ID 的 SignalChannel 实例
    ///
    /// 用于测试群组消息路由功能。
    ///
    /// # 参数
    /// - `group_id`: 群组标识符（"dm" 表示仅接受直接消息）
    ///
    /// # 返回值
    /// 配置了群组 ID 和通配符允许列表的 SignalChannel 实例
    fn make_channel_with_group(group_id: &str) -> SignalChannel {
        SignalChannel::new(
            "http://127.0.0.1:8686".to_string(),
            "+1234567890".to_string(),
            Some(group_id.to_string()),
            vec!["*".to_string()],
            true,
            true,
        )
    }

    /// 创建一个测试用的消息信封（Envelope）
    ///
    /// # 参数
    /// - `source_number`: 可选的发送者电话号码（E.164 格式）
    /// - `message`: 可选的消息文本内容
    ///
    /// # 返回值
    /// 包含指定发送者和消息的信封实例
    ///
    /// # 示例
    /// ```
    /// let env = make_envelope(Some("+1111111111"), Some("Hello"));
    /// ```
    fn make_envelope(source_number: Option<&str>, message: Option<&str>) -> Envelope {
        Envelope {
            source: source_number.map(String::from),
            source_number: source_number.map(String::from),
            data_message: message.map(|m| DataMessage {
                message: Some(m.to_string()),
                timestamp: Some(1_700_000_000_000),
                group_info: None,
                attachments: None,
            }),
            story_message: None,
            timestamp: Some(1_700_000_000_000),
        }
    }

    /// 测试 SignalChannel 创建时的字段初始化
    ///
    /// 验证所有配置参数都正确地存储在通道实例中。
    #[test]
    fn creates_with_correct_fields() {
        let ch = make_channel();
        assert_eq!(ch.http_url, "http://127.0.0.1:8686");
        assert_eq!(ch.account, "+1234567890");
        assert!(ch.group_id.is_none());
        assert_eq!(ch.allowed_from.len(), 1);
        assert!(!ch.ignore_attachments);
        assert!(!ch.ignore_stories);
    }

    /// 测试 URL 末尾斜杠的自动去除
    ///
    /// SignalChannel 应该自动去除 URL 末尾的斜杠，
    /// 以避免拼接 API 路径时出现双斜杠问题。
    #[test]
    fn strips_trailing_slash() {
        let ch = SignalChannel::new(
            "http://127.0.0.1:8686/".to_string(),
            "+1234567890".to_string(),
            None,
            vec![],
            false,
            false,
        );
        assert_eq!(ch.http_url, "http://127.0.0.1:8686");
    }

    /// 测试通配符 "*" 允许任何发送者
    ///
    /// 当允许列表包含 "*" 时，任何电话号码都应该被允许。
    #[test]
    fn wildcard_allows_anyone() {
        let ch = make_channel_with_group("dm");
        assert!(ch.is_sender_allowed("+9999999999"));
    }

    /// 测试特定发送者在允许列表中时被接受
    ///
    /// 发送者号码在 allowed_from 列表中时应该返回 true。
    #[test]
    fn specific_sender_allowed() {
        let ch = make_channel();
        assert!(ch.is_sender_allowed("+1111111111"));
    }

    /// 测试未知发送者被拒绝
    ///
    /// 不在允许列表中的发送者应该被拒绝。
    #[test]
    fn unknown_sender_denied() {
        let ch = make_channel();
        assert!(!ch.is_sender_allowed("+9999999999"));
    }

    /// 测试空允许列表拒绝所有发送者
    ///
    /// 当 allowed_from 为空时，任何发送者都应该被拒绝，
    /// 这是一个安全的默认行为。
    #[test]
    fn empty_allowlist_denies_all() {
        let ch = SignalChannel::new(
            "http://127.0.0.1:8686".to_string(),
            "+1234567890".to_string(),
            None,
            vec![],
            false,
            false,
        );
        assert!(!ch.is_sender_allowed("+1111111111"));
    }

    /// 测试 name() 方法返回 "signal"
    ///
    /// 通道名称应该始终返回小写的 "signal"。
    #[test]
    fn name_returns_signal() {
        let ch = make_channel();
        assert_eq!(ch.name(), "signal");
    }

    #[tokio::test]
    async fn rpc_request_returns_result_and_records_json_rpc_body() {
        let server = TestServer::spawn(vec![ResponseSpec::ok(r#"{"result":{"ok":true}}"#)]).await;
        let ch = SignalChannel::new(
            server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );

        let result = ch
            .rpc_request("send", serde_json::json!({"message": "hi"}))
            .await
            .expect("rpc request");

        assert_eq!(result, Some(serde_json::json!({"ok": true})));
        let requests = server.requests().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, Method::POST);
        assert_eq!(requests[0].path, "/api/v1/rpc");
        assert_eq!(requests[0].body["jsonrpc"], "2.0");
        assert_eq!(requests[0].body["method"], "send");
        assert_eq!(requests[0].body["params"]["message"], "hi");
        assert!(requests[0].body["id"].as_str().is_some());
    }

    #[tokio::test]
    async fn rpc_request_treats_created_and_empty_body_as_none() {
        let created_server = TestServer::spawn(vec![ResponseSpec::created()]).await;
        let created_channel = SignalChannel::new(
            created_server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );
        assert_eq!(
            created_channel.rpc_request("sendTyping", serde_json::json!({})).await.unwrap(),
            None
        );

        let empty_server = TestServer::spawn(vec![ResponseSpec::ok("")]).await;
        let empty_channel = SignalChannel::new(
            empty_server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );
        assert_eq!(empty_channel.rpc_request("send", serde_json::json!({})).await.unwrap(), None);
    }

    #[tokio::test]
    async fn rpc_request_reports_signal_rpc_error() {
        let server = TestServer::spawn(vec![ResponseSpec::ok(
            r#"{"error":{"code":-32602,"message":"bad params"}}"#,
        )])
        .await;
        let ch = SignalChannel::new(
            server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );

        let error = ch
            .rpc_request("send", serde_json::json!({}))
            .await
            .expect_err("rpc error should fail")
            .to_string();

        assert!(error.contains("Signal RPC error -32602: bad params"));
    }

    #[tokio::test]
    async fn send_and_start_typing_build_direct_and_group_params() {
        let server = TestServer::spawn(vec![
            ResponseSpec::ok(r#"{"result":true}"#),
            ResponseSpec::created(),
        ])
        .await;
        let ch = SignalChannel::new(
            server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );

        ch.send(&SendMessage::new("hello", "+1111111111")).await.expect("send direct");
        ch.start_typing("group:group-1").await.expect("typing group");

        let requests = server.requests().await;
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].body["method"], "send");
        assert_eq!(requests[0].body["params"]["recipient"], serde_json::json!(["+1111111111"]));
        assert_eq!(requests[0].body["params"]["account"], "+1234567890");
        assert_eq!(requests[1].body["method"], "sendTyping");
        assert_eq!(requests[1].body["params"]["groupId"], "group-1");
    }

    #[tokio::test]
    async fn health_check_reflects_http_status_and_stop_typing_is_noop() {
        let ok_server = TestServer::spawn(vec![ResponseSpec::ok("ok")]).await;
        let ok_channel = SignalChannel::new(
            ok_server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );
        assert!(ok_channel.health_check().await);
        assert_eq!(ok_server.requests().await[0].path, "/api/v1/check");
        ok_channel.stop_typing("+1111111111").await.expect("stop typing");

        let bad_server = TestServer::spawn(vec![ResponseSpec {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: "down",
        }])
        .await;
        let bad_channel = SignalChannel::new(
            bad_server.base_url.clone(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );
        assert!(!bad_channel.health_check().await);
    }

    #[tokio::test]
    async fn listen_rejects_invalid_event_stream_url_before_connecting() {
        let ch = SignalChannel::new(
            "://invalid".to_string(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );
        let (tx, _rx) = mpsc::channel(1);

        let error = ch.listen(tx).await.expect_err("invalid URL should be rejected").to_string();

        assert!(error.contains("relative URL without a base"));
    }

    /// 测试无群组 ID 时接受所有消息
    ///
    /// 当通道未配置群组 ID 时，应该接受：
    /// 1. 直接消息（无 group_info）
    /// 2. 任何群组消息
    #[test]
    fn matches_group_no_group_id_accepts_all() {
        let ch = make_channel();
        // 测试直接消息
        let dm = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: None,
            attachments: None,
        };
        assert!(ch.matches_group(&dm));

        // 测试群组消息
        let group = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: Some(GroupInfo { group_id: Some("group123".to_string()) }),
            attachments: None,
        };
        assert!(ch.matches_group(&group));
    }

    /// 测试群组 ID 过滤功能
    ///
    /// 当通道配置了群组 ID 时，应该：
    /// - 接受来自匹配群组的消息
    /// - 拒绝来自其他群组的消息
    #[test]
    fn matches_group_filters_group() {
        let ch = make_channel_with_group("group123");

        // 匹配的群组消息
        let matching = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: Some(GroupInfo { group_id: Some("group123".to_string()) }),
            attachments: None,
        };
        assert!(ch.matches_group(&matching));

        // 不匹配的群组消息
        let non_matching = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: Some(GroupInfo { group_id: Some("other_group".to_string()) }),
            attachments: None,
        };
        assert!(!ch.matches_group(&non_matching));
    }

    /// 测试 "dm" 关键字过滤直接消息
    ///
    /// 当群组 ID 设置为 "dm" 时，应该：
    /// - 接受直接消息（无 group_info）
    /// - 拒绝群组消息
    #[test]
    fn matches_group_dm_keyword() {
        let ch = make_channel_with_group("dm");

        // 直接消息应该被接受
        let dm = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: None,
            attachments: None,
        };
        assert!(ch.matches_group(&dm));

        // 群组消息应该被拒绝
        let group = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: Some(GroupInfo { group_id: Some("group123".to_string()) }),
            attachments: None,
        };
        assert!(!ch.matches_group(&group));
    }

    /// 测试直接消息的回复目标
    ///
    /// 对于直接消息，回复目标应该是发送者的电话号码。
    #[test]
    fn reply_target_dm() {
        let ch = make_channel();
        let dm = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: None,
            attachments: None,
        };
        assert_eq!(ch.reply_target(&dm, "+1111111111"), "+1111111111");
    }

    /// 测试群组消息的回复目标
    ///
    /// 对于群组消息，回复目标应该是 "group:" 前缀加上群组 ID。
    #[test]
    fn reply_target_group() {
        let ch = make_channel();
        let group = DataMessage {
            message: Some("hi".to_string()),
            timestamp: Some(1000),
            group_info: Some(GroupInfo { group_id: Some("group123".to_string()) }),
            attachments: None,
        };
        assert_eq!(ch.reply_target(&group, "+1111111111"), "group:group123");
    }

    /// 测试解析 E.164 格式号码为直接接收者
    ///
    /// 以 "+" 开头的数字字符串应该被解析为 Direct 接收者。
    #[test]
    fn parse_recipient_target_e164_is_direct() {
        assert_eq!(
            SignalChannel::parse_recipient_target("+1234567890"),
            RecipientTarget::Direct("+1234567890".to_string())
        );
    }

    /// 测试解析 "group:" 前缀为群组接收者
    ///
    /// 以 "group:" 开头的字符串应该被解析为 Group 接收者。
    #[test]
    fn parse_recipient_target_prefixed_group_is_group() {
        assert_eq!(
            SignalChannel::parse_recipient_target("group:abc123"),
            RecipientTarget::Group("abc123".to_string())
        );
    }

    /// 测试解析 UUID 为直接接收者
    ///
    /// 标准 UUID 格式应该被解析为 Direct 接收者，
    /// 用于支持 Signal 隐私模式用户。
    #[test]
    fn parse_recipient_target_uuid_is_direct() {
        let uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        assert_eq!(
            SignalChannel::parse_recipient_target(uuid),
            RecipientTarget::Direct(uuid.to_string())
        );
    }

    /// 测试非 E.164 格式的 "+" 前缀字符串为群组
    ///
    /// 以 "+" 开头但不是纯数字的字符串应该被解析为 Group 接收者。
    #[test]
    fn parse_recipient_target_non_e164_plus_is_group() {
        assert_eq!(
            SignalChannel::parse_recipient_target("+abc123"),
            RecipientTarget::Group("+abc123".to_string())
        );
    }

    #[test]
    fn parse_recipient_target_plain_group_id_is_group() {
        assert_eq!(
            SignalChannel::parse_recipient_target("plain-group-id"),
            RecipientTarget::Group("plain-group-id".to_string())
        );
    }

    #[test]
    fn is_e164_validates_prefix_length_and_digits() {
        assert!(SignalChannel::is_e164("+12"));
        assert!(SignalChannel::is_e164("+123456789012345"));
        assert!(!SignalChannel::is_e164("1234567890"));
        assert!(!SignalChannel::is_e164("+1"));
        assert!(!SignalChannel::is_e164("+1234567890123456"));
        assert!(!SignalChannel::is_e164("+123abc"));
    }

    /// 测试有效 UUID 格式的识别
    ///
    /// 标准的 UUID v4 格式应该返回 true。
    #[test]
    fn is_uuid_valid() {
        assert!(SignalChannel::is_uuid("a1b2c3d4-e5f6-7890-abcd-ef1234567890"));
        assert!(SignalChannel::is_uuid("00000000-0000-0000-0000-000000000000"));
    }

    /// 测试无效 UUID 格式的识别
    ///
    /// 以下格式应该返回 false：
    /// - E.164 电话号码
    /// - 随机字符串
    /// - 带前缀的字符串
    /// - 空字符串
    #[test]
    fn is_uuid_invalid() {
        assert!(!SignalChannel::is_uuid("+1234567890"));
        assert!(!SignalChannel::is_uuid("not-a-uuid"));
        assert!(!SignalChannel::is_uuid("group:abc123"));
        assert!(!SignalChannel::is_uuid(""));
    }

    /// 测试发送者提取优先使用 source_number
    ///
    /// 当信封同时包含 source 和 source_number 时，
    /// 应该优先使用 source_number（电话号码格式）。
    #[test]
    fn sender_prefers_source_number() {
        let env = Envelope {
            source: Some("uuid-123".to_string()),
            source_number: Some("+1111111111".to_string()),
            data_message: None,
            story_message: None,
            timestamp: Some(1000),
        };
        assert_eq!(SignalChannel::sender(&env), Some("+1111111111".to_string()));
    }

    /// 测试发送者提取回退到 source
    ///
    /// 当 source_number 不存在时，应该回退使用 source 字段。
    #[test]
    fn sender_falls_back_to_source() {
        let env = Envelope {
            source: Some("uuid-123".to_string()),
            source_number: None,
            data_message: None,
            story_message: None,
            timestamp: Some(1000),
        };
        assert_eq!(SignalChannel::sender(&env), Some("uuid-123".to_string()));
    }

    /// 测试 UUID 发送者的直接消息处理
    ///
    /// 验证隐私模式用户（使用 UUID 而非电话号码）发送的直接消息
    /// 能够被正确处理，包括：
    /// - 发送者识别为 UUID
    /// - 回复目标设置为 UUID（Direct 路由）
    /// - 消息内容正确提取
    #[test]
    fn process_envelope_uuid_sender_dm() {
        let uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let ch = SignalChannel::new(
            "http://127.0.0.1:8686".to_string(),
            "+1234567890".to_string(),
            None,
            vec!["*".to_string()],
            false,
            false,
        );
        let env = Envelope {
            source: Some(uuid.to_string()),
            source_number: None,
            data_message: Some(DataMessage {
                message: Some("Hello from privacy user".to_string()),
                timestamp: Some(1_700_000_000_000),
                group_info: None,
                attachments: None,
            }),
            story_message: None,
            timestamp: Some(1_700_000_000_000),
        };
        let msg = ch.process_envelope(&env).unwrap();
        assert_eq!(msg.sender, uuid);
        assert_eq!(msg.reply_target, uuid);
        assert_eq!(msg.content, "Hello from privacy user");

        // 验证回复路由：UUID 发送者的直接消息应该路由为 Direct 类型
        let target = SignalChannel::parse_recipient_target(&msg.reply_target);
        assert_eq!(target, RecipientTarget::Direct(uuid.to_string()));
    }

    /// 测试 UUID 发送者在群组中的消息处理
    ///
    /// 验证隐私模式用户在群组中发送的消息能够被正确处理：
    /// - 发送者识别为 UUID
    /// - 回复目标设置为群组 ID（Group 路由）
    /// - 群组消息应该回复到整个群组，而不是个人
    #[test]
    fn process_envelope_uuid_sender_in_group() {
        let uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let ch = SignalChannel::new(
            "http://127.0.0.1:8686".to_string(),
            "+1234567890".to_string(),
            Some("testgroup".to_string()),
            vec!["*".to_string()],
            false,
            false,
        );
        let env = Envelope {
            source: Some(uuid.to_string()),
            source_number: None,
            data_message: Some(DataMessage {
                message: Some("Group msg from privacy user".to_string()),
                timestamp: Some(1_700_000_000_000),
                group_info: Some(GroupInfo { group_id: Some("testgroup".to_string()) }),
                attachments: None,
            }),
            story_message: None,
            timestamp: Some(1_700_000_000_000),
        };
        let msg = ch.process_envelope(&env).unwrap();
        assert_eq!(msg.sender, uuid);
        assert_eq!(msg.reply_target, "group:testgroup");

        // 验证回复路由：群组消息应该路由为 Group 类型
        let target = SignalChannel::parse_recipient_target(&msg.reply_target);
        assert_eq!(target, RecipientTarget::Group("testgroup".to_string()));
    }

    /// 测试 source 和 source_number 都缺失时发送者为 None
    ///
    /// 当信封中既没有 source 也没有 source_number 时，
    /// sender() 方法应该返回 None。
    #[test]
    fn sender_none_when_both_missing() {
        let env = Envelope {
            source: None,
            source_number: None,
            data_message: None,
            story_message: None,
            timestamp: None,
        };
        assert_eq!(SignalChannel::sender(&env), None);
    }

    /// 测试处理有效的直接消息信封
    ///
    /// 验证来自允许发送者的有效消息能够被正确处理：
    /// - 消息内容正确提取
    /// - 发送者正确识别
    /// - 通道名称设置为 "signal"
    #[test]
    fn process_envelope_valid_dm() {
        let ch = make_channel();
        let env = make_envelope(Some("+1111111111"), Some("Hello!"));
        let msg = ch.process_envelope(&env).unwrap();
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.sender, "+1111111111");
        assert_eq!(msg.channel, "signal");
    }

    /// 测试拒绝被拒绝发送者的消息
    ///
    /// 来自不在允许列表中的发送者的消息应该返回 None。
    #[test]
    fn process_envelope_denied_sender() {
        let ch = make_channel();
        let env = make_envelope(Some("+9999999999"), Some("Hello!"));
        assert!(ch.process_envelope(&env).is_none());
    }

    /// 测试空消息被过滤
    ///
    /// 空字符串消息应该被过滤掉，返回 None。
    #[test]
    fn process_envelope_empty_message() {
        let ch = make_channel();
        let env = make_envelope(Some("+1111111111"), Some(""));
        assert!(ch.process_envelope(&env).is_none());
    }

    /// 测试无数据消息的信封被过滤
    ///
    /// 没有 data_message 的信封应该返回 None。
    #[test]
    fn process_envelope_no_data_message() {
        let ch = make_channel();
        let env = make_envelope(Some("+1111111111"), None);
        assert!(ch.process_envelope(&env).is_none());
    }

    /// 测试跳过故事消息
    ///
    /// 包含 story_message 的信封应该被过滤掉，
    /// 即使有有效的 data_message。
    #[test]
    fn process_envelope_skips_stories() {
        let ch = make_channel_with_group("dm");
        let mut env = make_envelope(Some("+1111111111"), Some("story text"));
        env.story_message = Some(serde_json::json!({}));
        assert!(ch.process_envelope(&env).is_none());
    }

    #[test]
    fn process_envelope_accepts_story_when_not_ignored() {
        let ch = make_channel();
        let mut env = make_envelope(Some("+1111111111"), Some("story text"));
        env.story_message = Some(serde_json::json!({"timestamp": 1_700_000_000_000u64}));

        let msg = ch.process_envelope(&env).expect("story is allowed when not ignored");

        assert_eq!(msg.content, "story text");
        assert_eq!(msg.sender, "+1111111111");
    }

    /// 测试跳过仅包含附件的消息
    ///
    /// 当消息没有文本内容只有附件时，应该被过滤掉。
    /// 这种消息通常需要不同的处理逻辑（如下载附件）。
    #[test]
    fn process_envelope_skips_attachment_only() {
        let ch = make_channel_with_group("dm");
        let env = Envelope {
            source: Some("+1111111111".to_string()),
            source_number: Some("+1111111111".to_string()),
            data_message: Some(DataMessage {
                message: None,
                timestamp: Some(1_700_000_000_000),
                group_info: None,
                attachments: Some(vec![serde_json::json!({"contentType": "image/png"})]),
            }),
            story_message: None,
            timestamp: Some(1_700_000_000_000),
        };
        assert!(ch.process_envelope(&env).is_none());
    }

    #[test]
    fn process_envelope_keeps_attachment_with_text() {
        let ch = make_channel_with_group("dm");
        let env = Envelope {
            source: Some("+1111111111".to_string()),
            source_number: Some("+1111111111".to_string()),
            data_message: Some(DataMessage {
                message: Some("caption".to_string()),
                timestamp: Some(1_700_000_000_000),
                group_info: None,
                attachments: Some(vec![serde_json::json!({"contentType": "image/png"})]),
            }),
            story_message: None,
            timestamp: Some(1_700_000_000_000),
        };

        let msg = ch.process_envelope(&env).expect("text keeps attachment message");

        assert_eq!(msg.content, "caption");
        assert_eq!(msg.reply_target, "+1111111111");
    }

    #[test]
    fn process_envelope_uses_current_time_when_no_timestamp_present() {
        let ch = make_channel();
        let env = Envelope {
            source: Some("+1111111111".to_string()),
            source_number: Some("+1111111111".to_string()),
            data_message: Some(DataMessage {
                message: Some("untimed".to_string()),
                timestamp: None,
                group_info: None,
                attachments: None,
            }),
            story_message: None,
            timestamp: None,
        };

        let msg = ch.process_envelope(&env).expect("message without timestamp");

        assert!(msg.id.starts_with("sig_"));
        assert!(msg.timestamp > 0);
        assert_eq!(msg.content, "untimed");
    }

    /// 测试 SSE 信封的反序列化
    ///
    /// 验证从 JSON 格式的 SSE 事件中正确解析出：
    /// - 发送者电话号码
    /// - 消息内容
    /// - 时间戳
    #[test]
    fn sse_envelope_deserializes() {
        let json = r#"{
                "envelope": {
                    "source": "+1111111111",
                    "sourceNumber": "+1111111111",
                    "timestamp": 1700000000000,
                    "dataMessage": {
                        "message": "Hello Signal!",
                        "timestamp": 1700000000000
                    }
                }
            }"#;
        let sse: SseEnvelope = serde_json::from_str(json).unwrap();
        let env = sse.envelope.unwrap();
        assert_eq!(env.source_number.as_deref(), Some("+1111111111"));
        let dm = env.data_message.unwrap();
        assert_eq!(dm.message.as_deref(), Some("Hello Signal!"));
    }

    /// 测试带群组信息的 SSE 信封反序列化
    ///
    /// 验证群组消息的 SSE 事件能够正确解析群组 ID。
    #[test]
    fn sse_envelope_deserializes_group() {
        let json = r#"{
                "envelope": {
                    "sourceNumber": "+2222222222",
                    "dataMessage": {
                        "message": "Group msg",
                        "groupInfo": {
                            "groupId": "abc123"
                        }
                    }
                }
            }"#;
        let sse: SseEnvelope = serde_json::from_str(json).unwrap();
        let env = sse.envelope.unwrap();
        let dm = env.data_message.unwrap();
        assert_eq!(dm.group_info.as_ref().unwrap().group_id.as_deref(), Some("abc123"));
    }

    /// 测试 Envelope 结构体的默认值
    ///
    /// 空的 JSON 对象应该被解析为所有字段都是 None 的信封，
    /// 这确保了反序列化的健壮性。
    #[test]
    fn envelope_defaults() {
        let json = r#"{}"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        assert!(env.source.is_none());
        assert!(env.source_number.is_none());
        assert!(env.data_message.is_none());
        assert!(env.story_message.is_none());
        assert!(env.timestamp.is_none());
    }
}
