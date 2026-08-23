use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Bytes,
        extract::State,
        http::{HeaderMap, Method, StatusCode, Uri},
    };
    use std::{collections::VecDeque, sync::Arc};
    use tokio::sync::{Mutex, oneshot};

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: Method,
        path: String,
        authorization: Option<String>,
        content_type: Option<String>,
        body: serde_json::Value,
    }

    #[derive(Clone, Debug)]
    struct ResponseSpec {
        status: StatusCode,
        body: &'static str,
    }

    #[derive(Clone)]
    struct TestServerState {
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        responses: Arc<Mutex<VecDeque<ResponseSpec>>>,
    }

    struct TestServer {
        base_url: String,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        shutdown: Option<oneshot::Sender<()>>,
    }

    impl TestServer {
        async fn spawn(responses: Vec<ResponseSpec>) -> Self {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
            let state = TestServerState { requests: requests.clone(), responses };
            let app = Router::new().fallback(record_request).with_state(state);
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

            Self { base_url: format!("http://{addr}"), requests, shutdown: Some(shutdown_tx) }
        }

        async fn requests(&self) -> Vec<RecordedRequest> {
            self.requests.lock().await.clone()
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
        State(state): State<TestServerState>,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, String) {
        let parsed_body = if body.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&body).expect("request body should be json")
        };

        state.requests.lock().await.push(RecordedRequest {
            method,
            path: uri.path().to_string(),
            authorization: headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            content_type: headers
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned),
            body: parsed_body,
        });

        let response = state
            .responses
            .lock()
            .await
            .pop_front()
            .unwrap_or(ResponseSpec { status: StatusCode::OK, body: r#"{"ok":true}"# });
        (response.status, response.body.to_string())
    }

    fn make_channel() -> WatiChannel {
        WatiChannel {
            api_token: "test-token".into(),
            api_url: "https://live-mt-server.wati.io".into(),
            tenant_id: None,
            allowed_numbers: vec!["+1234567890".into()],
            client: reqwest::Client::new(),
        }
    }

    fn make_wildcard_channel() -> WatiChannel {
        WatiChannel {
            api_token: "test-token".into(),
            api_url: "https://live-mt-server.wati.io".into(),
            tenant_id: None,
            allowed_numbers: vec!["*".into()],
            client: reqwest::Client::new(),
        }
    }

    fn make_channel_with_api_url(api_url: String) -> WatiChannel {
        WatiChannel {
            api_token: "test-token".into(),
            api_url,
            tenant_id: Some("tenant1".into()),
            allowed_numbers: vec!["*".into()],
            client: reqwest::Client::new(),
        }
    }

    fn unused_api_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind unused port");
        let api_url = format!("http://{}", listener.local_addr().expect("local addr"));
        drop(listener);
        api_url
    }

    fn current_unix_seconds() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    #[test]
    fn wati_channel_name() {
        let ch = make_channel();
        assert_eq!(ch.name(), "wati");
    }

    #[test]
    fn wati_new_initializes_channel_config() {
        let ch = WatiChannel::new(
            "test-token".to_string(),
            "https://live-mt-server.wati.io".to_string(),
            Some("tenant1".to_string()),
            vec!["*".to_string()],
        );

        assert_eq!(ch.name(), "wati");
        assert!(ch.is_number_allowed("+1234567890"));
        assert_eq!(ch.build_target("+1234567890"), "tenant1:1234567890");
    }

    #[test]
    fn wati_number_allowed_exact() {
        let ch = make_channel();
        assert!(ch.is_number_allowed("+1234567890"));
        assert!(!ch.is_number_allowed("+9876543210"));
    }

    #[test]
    fn wati_number_allowed_wildcard() {
        let ch = make_wildcard_channel();
        assert!(ch.is_number_allowed("+1234567890"));
        assert!(ch.is_number_allowed("+9999999999"));
    }

    #[test]
    fn wati_number_allowed_empty() {
        let ch = WatiChannel {
            api_token: "tok".into(),
            api_url: "https://live-mt-server.wati.io".into(),
            tenant_id: None,
            allowed_numbers: vec![],
            client: reqwest::Client::new(),
        };
        assert!(!ch.is_number_allowed("+1234567890"));
    }

    #[test]
    fn wati_build_target_with_tenant() {
        let ch = WatiChannel {
            api_token: "tok".into(),
            api_url: "https://live-mt-server.wati.io".into(),
            tenant_id: Some("tenant1".into()),
            allowed_numbers: vec![],
            client: reqwest::Client::new(),
        };
        assert_eq!(ch.build_target("+1234567890"), "tenant1:1234567890");
    }

    #[test]
    fn wati_build_target_without_tenant() {
        let ch = make_channel();
        assert_eq!(ch.build_target("+1234567890"), "1234567890");
    }

    #[test]
    fn wati_build_target_already_prefixed() {
        let ch = WatiChannel {
            api_token: "tok".into(),
            api_url: "https://live-mt-server.wati.io".into(),
            tenant_id: Some("tenant1".into()),
            allowed_numbers: vec![],
            client: reqwest::Client::new(),
        };
        // If the phone already has the tenant prefix, don't double it
        assert_eq!(ch.build_target("tenant1:1234567890"), "tenant1:1234567890");
    }

    #[test]
    fn wati_build_target_keeps_bare_number_without_plus() {
        let ch = make_channel();
        assert_eq!(ch.build_target("1234567890"), "1234567890");
    }

    #[test]
    fn wati_parse_valid_message() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "text": "Hello from WATI!",
            "waId": "1234567890",
            "fromMe": false,
            "timestamp": 1_705_320_000_u64
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
        assert_eq!(msgs[0].content, "Hello from WATI!");
        assert_eq!(msgs[0].channel, "wati");
        assert_eq!(msgs[0].reply_target, "+1234567890");
        assert_eq!(msgs[0].timestamp, 1_705_320_000);
    }

    #[test]
    fn wati_parse_skip_from_me() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": "My own message",
            "waId": "1234567890",
            "fromMe": true
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "fromMe messages should be skipped");
    }

    #[test]
    fn wati_parse_skip_no_text() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "waId": "1234567890",
            "fromMe": false
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "Messages without text should be skipped");
    }

    #[test]
    fn wati_parse_skip_blank_text_and_missing_sender() {
        let ch = make_wildcard_channel();
        let blank = serde_json::json!({
            "text": "   ",
            "waId": "1234567890",
            "fromMe": false
        });
        assert!(ch.parse_webhook_payload(&blank).is_empty());

        let missing_sender = serde_json::json!({
            "text": "hello",
            "fromMe": false
        });
        assert!(ch.parse_webhook_payload(&missing_sender).is_empty());
    }

    #[test]
    fn wati_parse_unauthorized_sender_skips_message() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "text": "Unauthorized",
            "waId": "9999999999",
            "fromMe": false
        });

        assert!(ch.parse_webhook_payload(&payload).is_empty());
    }

    #[test]
    fn wati_parse_alternative_field_names() {
        let ch = make_wildcard_channel();

        // wa_id instead of waId, message.body instead of text
        let payload = serde_json::json!({
            "message": { "body": "Alt field test" },
            "wa_id": "1234567890",
            "from_me": false,
            "timestamp": 1_705_320_000_u64
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Alt field test");
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    #[test]
    fn wati_parse_timestamp_seconds() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "timestamp": 1_705_320_000_u64
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs[0].timestamp, 1_705_320_000);
    }

    #[test]
    fn wati_parse_timestamp_milliseconds() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "timestamp": 1_705_320_000_000_u64
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs[0].timestamp, 1_705_320_000);
    }

    #[test]
    fn wati_parse_timestamp_iso() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "timestamp": "2025-01-15T12:00:00Z"
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs[0].timestamp, 1_736_942_400);
    }

    #[test]
    fn wati_parse_timestamp_invalid_iso_uses_current_time() {
        let ch = make_wildcard_channel();
        let start = current_unix_seconds();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "timestamp": "not-a-date"
        });

        let msgs = ch.parse_webhook_payload(&payload);
        let end = current_unix_seconds();
        assert!(msgs[0].timestamp >= start);
        assert!(msgs[0].timestamp <= end);
    }

    #[test]
    fn wati_parse_timestamp_non_numeric_json_uses_current_time() {
        let ch = make_wildcard_channel();
        let start = current_unix_seconds();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "timestamp": true
        });

        let msgs = ch.parse_webhook_payload(&payload);
        let end = current_unix_seconds();
        assert!(msgs[0].timestamp >= start);
        assert!(msgs[0].timestamp <= end);
    }

    #[test]
    fn wati_parse_missing_timestamp_uses_current_time() {
        let ch = make_wildcard_channel();
        let start = current_unix_seconds();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890"
        });

        let msgs = ch.parse_webhook_payload(&payload);
        let end = current_unix_seconds();
        assert!(msgs[0].timestamp >= start);
        assert!(msgs[0].timestamp <= end);
    }

    #[test]
    fn wati_parse_created_timestamp_fallbacks() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": "Created timestamp",
            "waId": "1234567890",
            "created": 1_705_320_000_000_u64
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].timestamp, 1_705_320_000);
    }

    #[test]
    fn wati_parse_normalizes_phone() {
        let ch = WatiChannel {
            api_token: "tok".into(),
            api_url: "https://live-mt-server.wati.io".into(),
            tenant_id: None,
            allowed_numbers: vec!["+1234567890".into()],
            client: reqwest::Client::new(),
        };

        // Phone without + prefix
        let payload = serde_json::json!({
            "text": "Hi",
            "waId": "1234567890",
            "fromMe": false
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    #[test]
    fn wati_parse_preserves_plus_prefixed_phone() {
        let ch = make_channel();
        let payload = serde_json::json!({
            "text": "Hi",
            "waId": "+1234567890",
            "fromMe": false
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    #[test]
    fn wati_parse_empty_payload() {
        let ch = make_channel();
        let payload = serde_json::json!({});
        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[test]
    fn wati_parse_from_field_fallback() {
        let ch = make_wildcard_channel();
        // Uses "from" instead of "waId"
        let payload = serde_json::json!({
            "text": "Fallback test",
            "from": "1234567890",
            "fromMe": false
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "+1234567890");
    }

    #[test]
    fn wati_parse_message_text_fallback() {
        let ch = make_wildcard_channel();
        // Uses "message.text" instead of top-level "text"
        let payload = serde_json::json!({
            "message": { "text": "Nested text" },
            "waId": "1234567890",
            "fromMe": false
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Nested text");
    }

    #[test]
    fn wati_parse_nested_body_and_created_iso_string() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": null,
            "message": { "body": "  Trimmed nested body  " },
            "waId": "1234567890",
            "created": "2025-01-15T12:00:00Z"
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Trimmed nested body");
        assert_eq!(msgs[0].timestamp, 1_736_942_400);
    }

    #[test]
    fn wati_parse_owner_field_as_from_me() {
        let ch = make_wildcard_channel();
        // Uses "owner" field as fromMe indicator
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "owner": true
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty(), "owner=true messages should be skipped");
    }

    #[test]
    fn wati_parse_from_me_field_fallback_skips_message() {
        let ch = make_wildcard_channel();
        let payload = serde_json::json!({
            "text": "Test",
            "waId": "1234567890",
            "from_me": true
        });

        let msgs = ch.parse_webhook_payload(&payload);
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn wati_send_posts_text_message() {
        let server = TestServer::spawn(vec![ResponseSpec {
            status: StatusCode::OK,
            body: r#"{"ok":true}"#,
        }])
        .await;
        let ch = make_channel_with_api_url(server.base_url.clone());

        ch.send(&SendMessage::new("hello", "+1234567890")).await.expect("send should succeed");

        let requests = server.requests().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, Method::POST);
        assert_eq!(requests[0].path, "/api/ext/v3/conversations/messages/text");
        assert_eq!(requests[0].authorization.as_deref(), Some("Bearer test-token"));
        assert_eq!(requests[0].content_type.as_deref(), Some("application/json"));
        assert_eq!(requests[0].body["target"], "tenant1:1234567890");
        assert_eq!(requests[0].body["text"], "hello");
    }

    #[tokio::test]
    async fn wati_send_returns_status_error() {
        let server = TestServer::spawn(vec![ResponseSpec {
            status: StatusCode::BAD_REQUEST,
            body: r#"{"error":"bad token test-token"}"#,
        }])
        .await;
        let ch = make_channel_with_api_url(server.base_url.clone());

        let err = ch
            .send(&SendMessage::new("hello", "+1234567890"))
            .await
            .expect_err("send should return status error");

        assert_eq!(err.to_string(), "WATI API error: 400 Bad Request");
    }

    #[tokio::test]
    async fn wati_send_preserves_existing_tenant_prefix() {
        let server = TestServer::spawn(vec![ResponseSpec {
            status: StatusCode::OK,
            body: r#"{"ok":true}"#,
        }])
        .await;
        let ch = make_channel_with_api_url(server.base_url.clone());

        ch.send(&SendMessage::new("hello", "tenant1:1234567890"))
            .await
            .expect("send should succeed");

        let requests = server.requests().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].body["target"], "tenant1:1234567890");
    }

    #[tokio::test]
    async fn wati_send_propagates_network_errors() {
        let ch = make_channel_with_api_url(unused_api_url());

        let err = ch
            .send(&SendMessage::new("hello", "+1234567890"))
            .await
            .expect_err("send should surface network failures");

        let err_text = err.to_string();
        assert!(
            err_text.contains("error sending request")
                || err_text.contains("tcp connect error")
                || err_text.contains("Connection refused"),
            "unexpected error: {err_text}"
        );
    }

    #[tokio::test]
    async fn wati_health_check_reflects_api_status() {
        let server = TestServer::spawn(vec![
            ResponseSpec { status: StatusCode::OK, body: r#"{"count":1}"# },
            ResponseSpec { status: StatusCode::INTERNAL_SERVER_ERROR, body: "error" },
        ])
        .await;
        let ch = make_channel_with_api_url(server.base_url.clone());

        assert!(ch.health_check().await);
        assert!(!ch.health_check().await);

        let requests = server.requests().await;
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, Method::GET);
        assert_eq!(requests[0].path, "/api/ext/v3/contacts/count");
        assert_eq!(requests[0].authorization.as_deref(), Some("Bearer test-token"));
        assert_eq!(requests[0].body, serde_json::Value::Null);
    }

    #[tokio::test]
    async fn wati_health_check_returns_false_on_network_error() {
        let ch = make_channel_with_api_url(unused_api_url());

        assert!(!ch.health_check().await);
    }

    #[tokio::test]
    async fn wati_typing_methods_are_noops() {
        let ch = make_channel();

        ch.start_typing("+1234567890").await.expect("start typing should succeed");
        ch.stop_typing("+1234567890").await.expect("stop typing should succeed");
    }

    #[tokio::test]
    async fn wati_listen_keeps_channel_alive_until_cancelled() {
        let ch = make_channel();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);

        let result =
            tokio::time::timeout(std::time::Duration::from_millis(10), ch.listen(tx)).await;

        assert!(result.is_err());
    }
}
// 本文件验证 WATI 渠道配置、号码许可、Webhook 解析与 HTTP 收发。
