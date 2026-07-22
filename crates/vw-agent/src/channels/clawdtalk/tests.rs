//! # ClawdTalk 通道测试模块
//!
//! 本模块包含 ClawdTalk 通道实现的单元测试，用于验证通道的创建、
//! 目的地访问控制以及 Webhook 事件反序列化等功能。
//!
//! ## 测试范围
//!
//! - 通道实例化与基本属性验证
//! - 目的地允许列表的精确匹配与通配符匹配逻辑
//! - Telnyx Webhook 事件的 JSON 反序列化

use super::*;

/// ClawdTalk 通道测试用例集合
///
/// 该模块包含所有与 ClawdTalk 通道相关的测试函数，
/// 涵盖配置验证、权限检查和事件解析等场景。
#[allow(dead_code)]
mod tests {
    use crate::app::agent::config::traits::ChannelConfig;

    use super::*;
    use std::time::Duration;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::oneshot,
    };

    /// 创建用于测试的 ClawdTalk 配置实例
    ///
    /// 返回一个预配置的 `ClawdTalkConfig` 实例，包含测试所需的
    /// API 密钥、连接 ID、发送号码以及允许的目的地前缀。
    ///
    /// # 返回值
    ///
    /// 返回一个 `ClawdTalkConfig` 实例，其中：
    /// - `api_key`: 测试用 API 密钥 "test-key"
    /// - `connection_id`: 测试用连接标识 "test-connection"
    /// - `from_number`: 发送方号码 "+15551234567"
    /// - `allowed_destinations`: 允许的目的地前缀列表 ["+1555"]
    /// - `webhook_secret`: 未设置（None）
    fn test_config() -> ClawdTalkConfig {
        ClawdTalkConfig {
            api_key: "test-key".to_string(),
            connection_id: "test-connection".to_string(),
            from_number: "+15551234567".to_string(),
            allowed_destinations: vec!["+1555".to_string()],
            webhook_secret: None,
        }
    }

    #[derive(Debug)]
    struct CapturedRequest {
        method: String,
        path: String,
        headers: String,
        body: String,
    }

    async fn spawn_telnyx_server(
        status: &'static str,
        body: &'static str,
    ) -> (String, oneshot::Receiver<CapturedRequest>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (request_tx, request_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let captured = read_captured_request(&mut stream).await;
            write_response(&mut stream, status, body).await;
            let _ = request_tx.send(captured);
        });

        (format!("http://{addr}/v2"), request_rx, handle)
    }

    async fn spawn_telnyx_sequence(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (String, mpsc::Receiver<CapturedRequest>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel(responses.len());
        let handle = tokio::spawn(async move {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let captured = read_captured_request(&mut stream).await;
                write_response(&mut stream, status, body).await;
                request_tx.send(captured).await.unwrap();
            }
        });

        (format!("http://{addr}/v2"), request_rx, handle)
    }

    async fn read_captured_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut buffer = Vec::new();
        let header_end = loop {
            let mut chunk = [0; 1024];
            let read = stream.read(&mut chunk).await.unwrap();
            assert_ne!(read, 0);
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(header_end) = find_header_end(&buffer) {
                break header_end;
            }
        };
        let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
        let content_length = content_length(&headers);
        let request_len = header_end + 4 + content_length;
        while buffer.len() < request_len {
            let mut chunk = [0; 1024];
            let read = stream.read(&mut chunk).await.unwrap();
            assert_ne!(read, 0);
            buffer.extend_from_slice(&chunk[..read]);
        }

        let request_line = headers.lines().next().unwrap();
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap().to_string();
        let path = parts.next().unwrap().to_string();
        let body_start = header_end + 4;
        CapturedRequest {
            method,
            path,
            headers,
            body: String::from_utf8_lossy(&buffer[body_start..request_len]).to_string(),
        }
    }

    async fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }

    fn find_header_end(buffer: &[u8]) -> Option<usize> {
        buffer.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn content_length(headers: &str) -> usize {
        header_value(headers, "content-length").and_then(|value| value.parse().ok()).unwrap_or(0)
    }

    fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
        headers.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if key.eq_ignore_ascii_case(name) { Some(value.trim()) } else { None }
        })
    }

    fn channel_with_api_url(api_url: String) -> ClawdTalkChannel {
        ClawdTalkChannel::new(test_config()).with_telnyx_api_url(api_url)
    }

    #[test]
    fn channel_config_metadata_is_stable() {
        assert_eq!(<ClawdTalkConfig as ChannelConfig>::name(), "ClawdTalk");
        assert_eq!(<ClawdTalkConfig as ChannelConfig>::desc(), "ClawdTalk Channel");
    }

    /// 测试 ClawdTalk 通道的创建
    ///
    /// 验证使用测试配置创建的通道实例能够正确返回其名称标识。
    /// 通道名称应为 "ClawdTalk"。
    #[test]
    fn creates_channel() {
        let channel = ClawdTalkChannel::new(test_config());
        assert_eq!(channel.name(), "ClawdTalk");
    }

    #[test]
    fn new_channel_uses_default_telnyx_api_url() {
        let channel = ClawdTalkChannel::new(test_config());

        assert_eq!(channel.telnyx_api_url(), ClawdTalkChannel::TELNYX_API_URL);
    }

    /// 测试目的地允许列表的精确前缀匹配
    ///
    /// 验证 `is_destination_allowed` 方法能够正确判断目的地号码
    /// 是否在允许列表中。使用前缀匹配逻辑，即目的地号码以
    /// 允许列表中的某个前缀开头则视为允许。
    ///
    /// # 测试用例
    ///
    /// - "+15559876543" 应被允许（以 "+1555" 开头）
    /// - "+14449876543" 应被拒绝（不以任何允许前缀开头）
    #[test]
    fn destination_allowed_exact_match() {
        let channel = ClawdTalkChannel::new(test_config());
        assert!(channel.is_destination_allowed("+15559876543"));
        assert!(!channel.is_destination_allowed("+14449876543"));
    }

    /// 测试通配符 "*" 允许所有目的地
    ///
    /// 当 `allowed_destinations` 包含通配符 "*" 时，
    /// 所有目的地号码都应被允许访问。
    ///
    /// # 测试用例
    ///
    /// - "+15559876543" 应被允许
    /// - "+14449876543" 应被允许
    #[test]
    fn destination_allowed_wildcard() {
        let mut config = test_config();
        config.allowed_destinations = vec!["*".to_string()];
        let channel = ClawdTalkChannel::new(config);
        assert!(channel.is_destination_allowed("+15559876543"));
        assert!(channel.is_destination_allowed("+14449876543"));
    }

    /// 测试空允许列表视为允许所有目的地
    ///
    /// 当 `allowed_destinations` 为空列表时，
    /// 应视为不限制访问，所有目的地号码都被允许。
    ///
    /// # 测试用例
    ///
    /// - "+15559876543" 应被允许
    /// - "+14449876543" 应被允许
    #[test]
    fn destination_allowed_empty_means_all() {
        let mut config = test_config();
        config.allowed_destinations = vec![];
        let channel = ClawdTalkChannel::new(config);
        assert!(channel.is_destination_allowed("+15559876543"));
        assert!(channel.is_destination_allowed("+14449876543"));
    }

    /// 测试 Telnyx Webhook 事件的 JSON 反序列化
    ///
    /// 验证 `TelnyxWebhookEvent` 结构体能够正确解析来自
    /// Telnyx 平台的 Webhook 事件 JSON 数据。
    ///
    /// # 测试数据
    ///
    /// 使用包含呼叫发起事件（call.initiated）的 JSON 数据，
    /// 包含完整的呼叫控制 ID、呼叫腿 ID、会话 ID、方向、
    /// 发送方和接收方号码以及呼叫状态等字段。
    #[test]
    fn webhook_event_deserializes() {
        // Telnyx Webhook 事件的 JSON 示例数据
        let json = r#"{
                "data": {
                    "event_type": "call.initiated",
                    "payload": {
                        "call_control_id": "call-123",
                        "call_leg_id": "leg-123",
                        "call_session_id": "session-123",
                        "direction": "incoming",
                        "from": "+15551112222",
                        "to": "+15553334444",
                        "state": "ringing"
                    }
                }
            }"#;

        // 反序列化 JSON 并验证关键字段
        let event: TelnyxWebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.data.event_type, "call.initiated");
        assert_eq!(event.data.payload.call_control_id, Some("call-123".to_string()));
        assert_eq!(event.data.payload.from, Some("+15551112222".to_string()));
    }

    #[test]
    fn channel_copies_config_and_supports_prefix_destination_match() {
        let mut config = test_config();
        config.allowed_destinations = vec!["+15551234567".to_string()];
        config.webhook_secret = Some("webhook-secret".to_string());
        let channel = ClawdTalkChannel::new(config);

        assert_eq!(channel.api_key, "test-key");
        assert_eq!(channel.connection_id, "test-connection");
        assert_eq!(channel.from_number, "+15551234567");
        assert_eq!(channel.webhook_secret.as_deref(), Some("webhook-secret"));
        assert!(channel.is_destination_allowed("+15551234567"));
        assert!(channel.is_destination_allowed("+155512345670"));
        assert!(!channel.is_destination_allowed("+15559876543"));
    }

    #[tokio::test]
    async fn initiate_call_rejects_disallowed_destination_before_network() {
        let channel = ClawdTalkChannel::new(test_config());
        let error =
            channel.initiate_call("+14445550123", Some("hello")).await.unwrap_err().to_string();

        assert!(error.contains("not in allowed list"));
        assert!(error.contains("+14445550123"));
    }

    #[tokio::test]
    async fn channel_send_propagates_destination_policy_errors() {
        let channel = ClawdTalkChannel::new(test_config());
        let error = Channel::send(
            &channel,
            &SendMessage {
                content: "hello".to_string(),
                recipient: "+14445550123".to_string(),
                subject: None,
                thread_ts: None,
            },
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("not in allowed list"));
    }

    #[test]
    fn call_request_skips_absent_optional_fields_and_keeps_present_fields() {
        let request = CallRequest {
            connection_id: "conn".to_string(),
            to: "+15550001111".to_string(),
            from: "+15552223333".to_string(),
            answering_machine_detection: Some(AnsweringMachineDetection {
                mode: "premium".to_string(),
            }),
            webhook_url: None,
            command_id: Some("cmd-1".to_string()),
        };

        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(value["connection_id"], "conn");
        assert_eq!(value["to"], "+15550001111");
        assert_eq!(value["from"], "+15552223333");
        assert_eq!(value["answering_machine_detection"]["mode"], "premium");
        assert_eq!(value["command_id"], "cmd-1");
        assert!(value.get("webhook_url").is_none());
    }

    #[test]
    fn speak_and_ai_payloads_serialize_expected_defaults() {
        let speak = SpeakRequest {
            payload: "hello".to_string(),
            payload_type: "text".to_string(),
            service_level: "premium".to_string(),
            voice: "female".to_string(),
            language: "en-US".to_string(),
        };
        let ai = AiConversationRequest {
            system_prompt: "answer briefly".to_string(),
            model: "gpt-4".to_string(),
            voice_settings: VoiceSettings { voice: "alloy".to_string(), speed: 1.0 },
        };

        let speak_value = serde_json::to_value(&speak).unwrap();
        let ai_value = serde_json::to_value(&ai).unwrap();

        assert_eq!(speak_value["payload"], "hello");
        assert_eq!(speak_value["payload_type"], "text");
        assert_eq!(speak_value["service_level"], "premium");
        assert_eq!(speak_value["voice"], "female");
        assert_eq!(speak_value["language"], "en-US");
        assert_eq!(ai_value["system_prompt"], "answer briefly");
        assert_eq!(ai_value["model"], "gpt-4");
        assert_eq!(ai_value["voice_settings"]["voice"], "alloy");
        assert_eq!(ai_value["voice_settings"]["speed"], 1.0);
    }

    #[test]
    fn call_response_and_session_clone_preserve_ids() {
        let response: CallResponse = serde_json::from_str(
            r#"{"call_control_id":"ctrl","call_leg_id":"leg","call_session_id":"session"}"#,
        )
        .unwrap();
        let session = CallSession {
            call_control_id: response.call_control_id,
            call_leg_id: response.call_leg_id,
            call_session_id: response.call_session_id,
        };
        let cloned = session.clone();

        assert_eq!(format!("{session:?}"), format!("{cloned:?}"));
        assert_eq!(cloned.call_control_id, "ctrl");
        assert_eq!(cloned.call_leg_id, "leg");
        assert_eq!(cloned.call_session_id, "session");
    }

    #[test]
    fn webhook_event_accepts_missing_optional_payload_fields() {
        let event: TelnyxWebhookEvent =
            serde_json::from_str(r#"{"data":{"event_type":"call.hangup","payload":{}}}"#).unwrap();

        assert_eq!(event.data.event_type, "call.hangup");
        assert!(event.data.payload.call_control_id.is_none());
        assert!(event.data.payload.call_leg_id.is_none());
        assert!(event.data.payload.call_session_id.is_none());
        assert!(event.data.payload.direction.is_none());
        assert!(event.data.payload.from.is_none());
        assert!(event.data.payload.to.is_none());
        assert!(event.data.payload.state.is_none());
    }

    #[tokio::test]
    async fn initiate_call_posts_expected_request_and_returns_session() {
        let (api_url, request_rx, handle) = spawn_telnyx_server(
            "200 OK",
            r#"{"call_control_id":"ctrl-1","call_leg_id":"leg-1","call_session_id":"session-1"}"#,
        )
        .await;
        let channel = channel_with_api_url(api_url);

        let session = channel.initiate_call("+15559876543", Some("ignored")).await.unwrap();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();

        assert_eq!(session.call_control_id, "ctrl-1");
        assert_eq!(session.call_leg_id, "leg-1");
        assert_eq!(session.call_session_id, "session-1");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v2/calls");
        assert_eq!(header_value(&request.headers, "authorization"), Some("Bearer test-key"));
        assert_eq!(header_value(&request.headers, "content-type"), Some("application/json"));
        assert_eq!(body["connection_id"], "test-connection");
        assert_eq!(body["to"], "+15559876543");
        assert_eq!(body["from"], "+15551234567");
        assert_eq!(body["answering_machine_detection"]["mode"], "premium");
        assert!(body.get("webhook_url").is_none());
        assert!(body.get("command_id").is_none());
    }

    #[tokio::test]
    async fn initiate_call_reports_api_error_body() {
        let (api_url, request_rx, handle) = spawn_telnyx_server("403 Forbidden", "denied").await;
        let channel = channel_with_api_url(api_url);

        let error = channel.initiate_call("+15559876543", None).await.unwrap_err().to_string();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();

        assert_eq!(request.path, "/v2/calls");
        assert!(error.contains("Failed to initiate call: denied"));
    }

    #[tokio::test]
    async fn speak_posts_premium_voice_payload() {
        let (api_url, request_rx, handle) = spawn_telnyx_server("200 OK", "{}").await;
        let channel = channel_with_api_url(api_url);

        channel.speak("ctrl-1", "hello").await.unwrap();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v2/calls/ctrl-1/actions/speak");
        assert_eq!(header_value(&request.headers, "authorization"), Some("Bearer test-key"));
        assert_eq!(body["payload"], "hello");
        assert_eq!(body["payload_type"], "text");
        assert_eq!(body["service_level"], "premium");
        assert_eq!(body["voice"], "female");
        assert_eq!(body["language"], "en-US");
    }

    #[tokio::test]
    async fn speak_reports_api_error_body() {
        let (api_url, request_rx, handle) =
            spawn_telnyx_server("422 Unprocessable Entity", "bad").await;
        let channel = channel_with_api_url(api_url);

        let error = channel.speak("ctrl-1", "hello").await.unwrap_err().to_string();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();

        assert_eq!(request.path, "/v2/calls/ctrl-1/actions/speak");
        assert!(error.contains("Failed to speak: bad"));
    }

    #[tokio::test]
    async fn hangup_posts_action_and_suppresses_api_error() {
        let (api_url, request_rx, handle) =
            spawn_telnyx_server("409 Conflict", "already-ended").await;
        let channel = channel_with_api_url(api_url);

        channel.hangup("ctrl-1").await.unwrap();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v2/calls/ctrl-1/actions/hangup");
        assert_eq!(header_value(&request.headers, "authorization"), Some("Bearer test-key"));
        assert!(request.body.is_empty());
    }

    #[tokio::test]
    async fn start_ai_conversation_posts_model_prompt_and_voice_settings() {
        let (api_url, request_rx, handle) = spawn_telnyx_server("200 OK", "{}").await;
        let channel = channel_with_api_url(api_url);

        channel.start_ai_conversation("ctrl-1", "system", "model-a").await.unwrap();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();
        let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v2/calls/ctrl-1/actions/ai_conversation");
        assert_eq!(body["system_prompt"], "system");
        assert_eq!(body["model"], "model-a");
        assert_eq!(body["voice_settings"]["voice"], "alloy");
        assert_eq!(body["voice_settings"]["speed"], 1.0);
    }

    #[tokio::test]
    async fn start_ai_conversation_reports_api_error_body() {
        let (api_url, request_rx, handle) =
            spawn_telnyx_server("500 Internal Server Error", "oops").await;
        let channel = channel_with_api_url(api_url);

        let error = channel
            .start_ai_conversation("ctrl-1", "system", "model-a")
            .await
            .unwrap_err()
            .to_string();
        let request = request_rx.await.unwrap();
        handle.await.unwrap();

        assert_eq!(request.path, "/v2/calls/ctrl-1/actions/ai_conversation");
        assert!(error.contains("Failed to start AI conversation: oops"));
    }

    #[tokio::test]
    async fn health_check_returns_status_success() {
        for (status, expected) in [("200 OK", true), ("401 Unauthorized", false)] {
            let (api_url, request_rx, handle) = spawn_telnyx_server(status, "{}").await;
            let channel = channel_with_api_url(api_url);

            assert_eq!(channel.health_check().await, expected);
            let request = request_rx.await.unwrap();
            handle.await.unwrap();

            assert_eq!(request.method, "GET");
            assert_eq!(request.path, "/v2/phone_numbers");
            assert_eq!(header_value(&request.headers, "authorization"), Some("Bearer test-key"));
        }
    }

    #[tokio::test]
    async fn health_check_returns_false_when_request_fails() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let channel = channel_with_api_url(format!("http://{addr}/v2"));

        assert!(!channel.health_check().await);
    }

    #[tokio::test]
    async fn channel_send_initiates_speaks_and_hangups_in_order() {
        let (api_url, mut request_rx, handle) = spawn_telnyx_sequence(vec![
            (
                "200 OK",
                r#"{"call_control_id":"ctrl-send","call_leg_id":"leg-send","call_session_id":"session-send"}"#,
            ),
            ("200 OK", "{}"),
            ("200 OK", "{}"),
        ])
        .await;
        let channel = channel_with_api_url(api_url);
        let message = SendMessage {
            content: "hello".to_string(),
            recipient: "+15559876543".to_string(),
            subject: None,
            thread_ts: None,
        };

        let send_handle = tokio::spawn(async move { Channel::send(&channel, &message).await });
        let call_request = request_rx.recv().await.unwrap();
        let speak_request = request_rx.recv().await.unwrap();
        let hangup_request = request_rx.recv().await.unwrap();

        tokio::time::timeout(Duration::from_secs(10), send_handle).await.unwrap().unwrap().unwrap();
        handle.await.unwrap();
        assert_eq!(call_request.path, "/v2/calls");
        assert_eq!(speak_request.path, "/v2/calls/ctrl-send/actions/speak");
        assert_eq!(hangup_request.path, "/v2/calls/ctrl-send/actions/hangup");
    }

    #[tokio::test(start_paused = true)]
    async fn listen_exits_after_sender_is_closed() {
        let channel = ClawdTalkChannel::new(test_config());
        let (tx, rx) = mpsc::channel(1);
        drop(rx);

        let handle = tokio::spawn(async move { Channel::listen(&channel, tx).await });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(60)).await;

        handle.await.unwrap().unwrap();
    }
}
