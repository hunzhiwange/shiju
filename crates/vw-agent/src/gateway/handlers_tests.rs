// 覆盖渠道 Webhook 处理器的鉴权、签名校验、代理与错误响应边界。

// 渠道 fixture 覆盖成功回复与降级回复，并验证外部签名先于载荷处理。
use super::*;
use crate::app::agent::channels::traits::{ChannelMessage, SendMessage};
use crate::app::agent::channels::{
    LinqChannel, NextcloudTalkChannel, QQChannel, WatiChannel, WhatsAppChannel,
};
use crate::app::agent::config::{
    Config, ProxyConfig, ProxyScope, runtime_proxy_config, set_runtime_proxy_config,
};
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::{Bytes, to_bytes};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use hmac::{Hmac, KeyInit, Mac};
use parking_lot::Mutex;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
struct RecordingChannel {
    sent: Mutex<Vec<SendMessage>>,
    fail_send: bool,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for RecordingChannel {
    fn name(&self) -> &str {
        "recording"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.sent.lock().push(message.clone());
        if self.fail_send {
            anyhow::bail!("send failed");
        }
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }
}

// 构造统一渠道消息，便于比较成功回复和错误降级是否保留线程上下文。
fn channel_message() -> ChannelMessage {
    ChannelMessage {
        instance_id: "default".to_string(),
        id: "message-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "target-1".to_string(),
        content: "hello".to_string(),
        channel: "recording".to_string(),
        timestamp: 123,
        thread_ts: Some("thread-1".to_string()),
    }
}

fn successful_gateway_chat<'a>(
    _state: &'a AppState,
    _message: &'a str,
    _key: &'a str,
) -> GatewayChatFuture<'a> {
    Box::pin(async { Ok("\nreply from injected runner\n".to_string()) })
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(format!("echo: {message}"))
    }
}

struct FailingProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for FailingProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("provider failed")
    }
}

fn state() -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1_000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: Default::default(),
        max_iterations: 5,
        event_tx: tokio::sync::broadcast::channel(1).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

async fn response_text(response: axum::response::Response) -> (StatusCode, String) {
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn response_json(response: axum::response::Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

// 签名由测试正文现算，避免把固定样例误当成对任意载荷都有效。
fn valid_whatsapp_signature(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

async fn spawn_whatsapp_graph_error_server() -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else {
            return;
        };
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await;
        let body = r#"{"error":{"message":"bad token"}}"#;
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes()).await;
    });
    format!("http://{addr}/v18.0")
}

fn valid_linq_signature(secret: &str, body: &str, timestamp: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{timestamp}.{body}").as_bytes());
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

fn valid_nextcloud_talk_signature(secret: &str, random: &str, body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{random}{body}").as_bytes());
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

#[tokio::test]
async fn process_channel_messages_sends_fallback_with_thread_when_gateway_chat_fails() {
    let mut state = state();
    state.provider = Arc::new(FailingProvider);
    state.auto_save = true;
    let channel = RecordingChannel::default();
    let messages = vec![channel_message()];

    process_channel_messages_with_chat(
        &state,
        "Recording",
        &channel,
        &messages,
        |msg| format!("recording:{}", msg.sender),
        "recording message",
        true,
        run_gateway_chat_with_tools_boxed,
    )
    .await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "target-1");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
    assert_eq!(sent[0].content, "Sorry, I couldn't process your message right now.");
}

#[tokio::test]
async fn process_channel_messages_with_chat_sends_successful_reply() {
    let state = state();
    let channel = RecordingChannel::default();
    let messages = vec![channel_message()];

    process_channel_messages_with_chat(
        &state,
        "Recording",
        &channel,
        &messages,
        |msg| format!("recording:{}", msg.sender),
        "recording message",
        true,
        successful_gateway_chat,
    )
    .await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "target-1");
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
    assert_eq!(sent[0].content, "reply from injected runner");
}

#[tokio::test]
async fn send_successful_channel_reply_sanitizes_and_omits_thread_when_disabled() {
    let channel = RecordingChannel::default();
    let msg = channel_message();
    let tools = Vec::new();

    send_successful_channel_reply(
        "Recording",
        &channel,
        &msg,
        "\n\nsuccessful reply\n\n",
        &tools,
        false,
    )
    .await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, "target-1");
    assert!(sent[0].thread_ts.is_none());
    assert_eq!(sent[0].content, "successful reply");
}

#[tokio::test]
async fn send_successful_channel_reply_logs_send_errors_without_panicking() {
    let channel = RecordingChannel { sent: Mutex::new(Vec::new()), fail_send: true };
    let msg = channel_message();
    let tools = Vec::new();

    send_successful_channel_reply("Recording", &channel, &msg, "reply", &tools, true).await;

    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].thread_ts.as_deref(), Some("thread-1"));
}

#[test]
fn whatsapp_signature_accepts_valid_hmac() {
    let signature = valid_whatsapp_signature("secret", b"body");
    assert!(verify_whatsapp_signature("secret", b"body", &signature));
}

#[test]
fn whatsapp_signature_rejects_wrong_signature() {
    assert!(!verify_whatsapp_signature("secret", b"body", "sha256=not-valid"));
}

#[test]
fn whatsapp_signature_rejects_missing_prefix() {
    assert!(!verify_whatsapp_signature("secret", b"body", "abc123"));
}

#[test]
fn whatsapp_signature_rejects_valid_hex_with_wrong_length() {
    assert!(!verify_whatsapp_signature("secret", b"body", "sha256=00"));
}

#[test]
fn whatsapp_verify_query_deserializes_fields() {
    let query: WhatsAppVerifyQuery = serde_json::from_value(serde_json::json!({
        "hub.mode": "subscribe",
        "hub.verify_token": "token",
        "hub.challenge": "challenge"
    }))
    .expect("valid query");

    assert_eq!(query.mode.as_deref(), Some("subscribe"));
    assert_eq!(query.verify_token.as_deref(), Some("token"));
    assert_eq!(query.challenge.as_deref(), Some("challenge"));
}

#[tokio::test]
async fn whatsapp_verify_returns_not_found_when_unconfigured() {
    let response = handle_whatsapp_verify(
        State(state()),
        Query(WhatsAppVerifyQuery {
            mode: Some("subscribe".into()),
            verify_token: Some("token".into()),
            challenge: Some("challenge".into()),
        }),
    )
    .await;

    let (status, body) = response_text(response).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("not configured"));
}

#[tokio::test]
async fn whatsapp_verify_echoes_challenge_for_matching_token() {
    let state = state();
    state.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
    )));

    let response = handle_whatsapp_verify(
        State(state),
        Query(WhatsAppVerifyQuery {
            mode: Some("subscribe".into()),
            verify_token: Some("verify-token".into()),
            challenge: Some("challenge-123".into()),
        }),
    )
    .await;

    assert_eq!(response_text(response).await, (StatusCode::OK, "challenge-123".into()));
}

#[tokio::test]
async fn whatsapp_verify_rejects_bad_or_incomplete_verification() {
    let state = state();
    state.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
    )));

    let forbidden = handle_whatsapp_verify(
        State(state.clone()),
        Query(WhatsAppVerifyQuery {
            mode: Some("subscribe".into()),
            verify_token: Some("wrong".into()),
            challenge: Some("challenge".into()),
        }),
    )
    .await;
    assert_eq!(response_text(forbidden).await.0, StatusCode::FORBIDDEN);

    let missing_challenge = handle_whatsapp_verify(
        State(state),
        Query(WhatsAppVerifyQuery {
            mode: Some("subscribe".into()),
            verify_token: Some("verify-token".into()),
            challenge: None,
        }),
    )
    .await;
    assert_eq!(response_text(missing_challenge).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn whatsapp_verify_requires_subscribe_mode_and_token() {
    let configured = state();
    configured.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
    )));

    let wrong_mode = handle_whatsapp_verify(
        State(configured.clone()),
        Query(WhatsAppVerifyQuery {
            mode: Some("unsubscribe".into()),
            verify_token: Some("verify-token".into()),
            challenge: Some("challenge".into()),
        }),
    )
    .await;
    assert_eq!(response_text(wrong_mode).await.0, StatusCode::FORBIDDEN);

    let missing_token = handle_whatsapp_verify(
        State(configured),
        Query(WhatsAppVerifyQuery {
            mode: Some("subscribe".into()),
            verify_token: None,
            challenge: Some("challenge".into()),
        }),
    )
    .await;
    assert_eq!(response_text(missing_token).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn whatsapp_message_validates_configuration_signature_json_and_empty_payloads() {
    let (status, body) = response_json(
        handle_whatsapp_message(State(state()), HeaderMap::new(), Bytes::new()).await,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "WhatsApp not configured");

    let configured = state();
    configured.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().whatsapp_app_secret = Some(Arc::from("secret"));

    let (status, body) = response_json(
        handle_whatsapp_message(State(configured.clone()), HeaderMap::new(), Bytes::from("{}"))
            .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid signature");

    let payload = br#"{"entry":[]}"#;
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Hub-Signature-256",
        HeaderValue::from_str(&valid_whatsapp_signature("secret", payload)).unwrap(),
    );
    let (status, body) = response_json(
        handle_whatsapp_message(State(configured.clone()), headers, Bytes::from_static(payload))
            .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    configured.channels.lock().whatsapp_app_secret = None;
    let (status, body) = response_json(
        handle_whatsapp_message(State(configured), HeaderMap::new(), Bytes::from("{")).await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");
}

#[tokio::test]
async fn whatsapp_message_valid_signature_does_not_hide_invalid_json() {
    let configured = state();
    configured.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().whatsapp_app_secret = Some(Arc::from("secret"));

    let payload = b"{";
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Hub-Signature-256",
        HeaderValue::from_str(&valid_whatsapp_signature("secret", payload)).unwrap(),
    );

    let (status, body) = response_json(
        handle_whatsapp_message(State(configured), headers, Bytes::from_static(payload)).await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");
}

#[tokio::test]
async fn whatsapp_message_rejects_present_invalid_signature() {
    let configured = state();
    configured.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().whatsapp_app_secret = Some(Arc::from("secret"));

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", HeaderValue::from_static("sha256=00"));

    let (status, body) =
        response_json(handle_whatsapp_message(State(configured), headers, Bytes::from("{}")).await)
            .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid signature");
}

#[tokio::test(flavor = "current_thread")]
async fn whatsapp_message_acknowledges_nonempty_payload_after_local_send_failure() {
    let _proxy_lock =
        crate::test_support::global_runtime_proxy_lock().lock().unwrap_or_else(|e| e.into_inner());
    let original_proxy = runtime_proxy_config();
    set_runtime_proxy_config(ProxyConfig::default());

    let configured = state();
    configured.channels.lock().whatsapp = Some(Arc::new(WhatsAppChannel::new_with_api_base_url(
        "access".into(),
        "phone-id".into(),
        "verify-token".into(),
        vec!["*".into()],
        spawn_whatsapp_graph_error_server().await,
    )));

    let payload = Bytes::from_static(
        br#"{"entry":[{"changes":[{"value":{"messages":[{"from":"15551230000","timestamp":"123","text":{"body":"hello"}}]}}]}]}"#,
    );
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        handle_whatsapp_message(State(configured), HeaderMap::new(), payload),
    )
    .await;

    set_runtime_proxy_config(original_proxy);

    let (status, body) = response_json(response.expect("whatsapp handler should return")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn linq_webhook_handles_configuration_signature_json_and_empty_messages() {
    let (status, _) =
        response_json(handle_linq_webhook(State(state()), HeaderMap::new(), Bytes::new()).await)
            .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let configured = state();
    configured.channels.lock().linq =
        Some(Arc::new(LinqChannel::new("token".into(), "+15550000000".into(), vec!["*".into()])));
    configured.channels.lock().linq_signing_secret = Some(Arc::from("secret"));

    let (status, body) = response_json(
        handle_linq_webhook(State(configured.clone()), HeaderMap::new(), Bytes::from("{}")).await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid signature");

    configured.channels.lock().linq_signing_secret = None;
    let (status, body) = response_json(
        handle_linq_webhook(State(configured.clone()), HeaderMap::new(), Bytes::from("{")).await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");

    let (status, body) = response_json(
        handle_linq_webhook(
            State(configured),
            HeaderMap::new(),
            Bytes::from(r#"{"event_type":"delivery.updated"}"#),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn linq_webhook_accepts_valid_signature_before_empty_message_ack() {
    let configured = state();
    configured.channels.lock().linq =
        Some(Arc::new(LinqChannel::new("token".into(), "+15550000000".into(), vec!["*".into()])));
    configured.channels.lock().linq_signing_secret = Some(Arc::from("secret"));

    let body = r#"{"event_type":"delivery.updated"}"#;
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let mut headers = HeaderMap::new();
    headers.insert("X-Webhook-Timestamp", HeaderValue::from_str(&timestamp).unwrap());
    headers.insert(
        "X-Webhook-Signature",
        HeaderValue::from_str(&valid_linq_signature("secret", body, &timestamp)).unwrap(),
    );

    let (status, body) =
        response_json(handle_linq_webhook(State(configured), headers, Bytes::from(body)).await)
            .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn linq_webhook_rejects_present_invalid_signature() {
    let configured = state();
    configured.channels.lock().linq =
        Some(Arc::new(LinqChannel::new("token".into(), "+15550000000".into(), vec!["*".into()])));
    configured.channels.lock().linq_signing_secret = Some(Arc::from("secret"));

    let mut headers = HeaderMap::new();
    headers.insert("X-Webhook-Timestamp", HeaderValue::from_static("123"));
    headers.insert("X-Webhook-Signature", HeaderValue::from_static("sha256=00"));

    let (status, body) =
        response_json(handle_linq_webhook(State(configured), headers, Bytes::from("{}")).await)
            .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid signature");
}

#[tokio::test]
async fn linq_webhook_acknowledges_unactionable_message_received_event() {
    let configured = state();
    configured.channels.lock().linq =
        Some(Arc::new(LinqChannel::new("token".into(), "+15550000000".into(), vec!["*".into()])));

    let (status, body) = response_json(
        handle_linq_webhook(
            State(configured),
            HeaderMap::new(),
            Bytes::from(r#"{"event_type":"message.received"}"#),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[test]
fn wati_verify_query_deserializes_challenge_and_defaults_missing_value() {
    let query: WatiVerifyQuery = serde_json::from_value(serde_json::json!({
        "hub.challenge": "challenge"
    }))
    .expect("valid query");
    assert_eq!(query.challenge.as_deref(), Some("challenge"));

    let query: WatiVerifyQuery =
        serde_json::from_value(serde_json::json!({})).expect("empty query");
    assert!(query.challenge.is_none());
}

#[tokio::test]
async fn wati_verify_and_webhook_cover_error_and_empty_paths() {
    let verify_missing = handle_wati_verify(
        State(state()),
        Query(WatiVerifyQuery { challenge: Some("challenge".into()) }),
    )
    .await;
    assert_eq!(response_text(verify_missing).await.0, StatusCode::NOT_FOUND);

    let configured = state();
    configured.channels.lock().wati = Some(Arc::new(WatiChannel::new(
        "token".into(),
        "https://wati.example".into(),
        None,
        vec!["*".into()],
    )));

    let verified = handle_wati_verify(
        State(configured.clone()),
        Query(WatiVerifyQuery { challenge: Some("wati-challenge".into()) }),
    )
    .await;
    assert_eq!(response_text(verified).await, (StatusCode::OK, "wati-challenge".into()));

    let missing_challenge =
        handle_wati_verify(State(configured.clone()), Query(WatiVerifyQuery { challenge: None }))
            .await;
    assert_eq!(response_text(missing_challenge).await.0, StatusCode::BAD_REQUEST);

    let (status, body) =
        response_json(handle_wati_webhook(State(state()), Bytes::new()).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "WATI not configured");

    let (status, body) =
        response_json(handle_wati_webhook(State(configured.clone()), Bytes::from("{")).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");

    let (status, body) =
        response_json(handle_wati_webhook(State(configured), Bytes::from("{}")).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn wati_verify_echoes_empty_challenge() {
    let configured = state();
    configured.channels.lock().wati = Some(Arc::new(WatiChannel::new(
        "token".into(),
        "https://wati.example".into(),
        None,
        vec!["*".into()],
    )));

    let response = handle_wati_verify(
        State(configured),
        Query(WatiVerifyQuery { challenge: Some(String::new()) }),
    )
    .await;
    assert_eq!(response_text(response).await, (StatusCode::OK, String::new()));
}

#[tokio::test]
async fn wati_webhook_processes_messages_and_handles_provider_errors() {
    let payload = Bytes::from_static(
        br#"{"text":"hello","waId":"15551230000","fromMe":false,"timestamp":123}"#,
    );

    let mut configured = state();
    configured.channels.lock().wati = Some(Arc::new(WatiChannel::new(
        "token".into(),
        "http://127.0.0.1:9".into(),
        Some("tenant".into()),
        vec!["*".into()],
    )));

    let (status, body) =
        response_json(handle_wati_webhook(State(configured.clone()), payload.clone()).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    configured.provider = Arc::new(FailingProvider);
    let (status, body) = response_json(handle_wati_webhook(State(configured), payload).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn nextcloud_talk_webhook_handles_security_and_empty_payloads() {
    let (status, _) = response_json(
        handle_nextcloud_talk_webhook(State(state()), HeaderMap::new(), Bytes::new()).await,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let configured = state();
    configured.channels.lock().nextcloud_talk = Some(Arc::new(NextcloudTalkChannel::new(
        "https://cloud.example".into(),
        "app-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().nextcloud_talk_webhook_secret = Some(Arc::from("secret"));

    let (status, body) = response_json(
        handle_nextcloud_talk_webhook(
            State(configured.clone()),
            HeaderMap::new(),
            Bytes::from("{}"),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid signature");

    configured.channels.lock().nextcloud_talk_webhook_secret = None;
    let (status, body) = response_json(
        handle_nextcloud_talk_webhook(
            State(configured.clone()),
            HeaderMap::new(),
            Bytes::from("{"),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");

    let (status, body) = response_json(
        handle_nextcloud_talk_webhook(
            State(configured),
            HeaderMap::new(),
            Bytes::from(r#"{"type":"delete"}"#),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn nextcloud_talk_webhook_accepts_valid_signature_and_processes_messages() {
    let body = r#"{"type":"message","message":{"token":"room-token","actorType":"users","actorId":"alice","messageType":"comment","message":"hello","id":"msg-1","timestamp":123}}"#;
    let random = "nonce";
    let mut headers = HeaderMap::new();
    headers.insert("X-Nextcloud-Talk-Random", HeaderValue::from_static(random));
    headers.insert(
        "X-Nextcloud-Talk-Signature",
        HeaderValue::from_str(&valid_nextcloud_talk_signature("secret", random, body)).unwrap(),
    );

    let mut configured = state();
    configured.channels.lock().nextcloud_talk = Some(Arc::new(NextcloudTalkChannel::new(
        "http://127.0.0.1:9".into(),
        "app-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().nextcloud_talk_webhook_secret = Some(Arc::from("secret"));

    let (status, body_json) = response_json(
        handle_nextcloud_talk_webhook(
            State(configured.clone()),
            headers.clone(),
            Bytes::from(body),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body_json["status"], "ok");

    configured.provider = Arc::new(FailingProvider);
    let (status, body_json) = response_json(
        handle_nextcloud_talk_webhook(State(configured), headers, Bytes::from(body)).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body_json["status"], "ok");
}

#[tokio::test]
async fn nextcloud_talk_valid_signature_does_not_hide_invalid_json() {
    let body = "{";
    let random = "nonce";
    let mut headers = HeaderMap::new();
    headers.insert("X-Nextcloud-Talk-Random", HeaderValue::from_static(random));
    headers.insert(
        "X-Nextcloud-Talk-Signature",
        HeaderValue::from_str(&valid_nextcloud_talk_signature("secret", random, body)).unwrap(),
    );

    let configured = state();
    configured.channels.lock().nextcloud_talk = Some(Arc::new(NextcloudTalkChannel::new(
        "https://cloud.example".into(),
        "app-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().nextcloud_talk_webhook_secret = Some(Arc::from("secret"));

    let (status, body_json) = response_json(
        handle_nextcloud_talk_webhook(State(configured), headers, Bytes::from(body)).await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body_json["error"], "Invalid JSON payload");
}

#[tokio::test]
async fn nextcloud_talk_webhook_rejects_present_invalid_signature() {
    let configured = state();
    configured.channels.lock().nextcloud_talk = Some(Arc::new(NextcloudTalkChannel::new(
        "https://cloud.example".into(),
        "app-token".into(),
        vec!["*".into()],
    )));
    configured.channels.lock().nextcloud_talk_webhook_secret = Some(Arc::from("secret"));

    let mut headers = HeaderMap::new();
    headers.insert("X-Nextcloud-Talk-Random", HeaderValue::from_static("nonce"));
    headers.insert("X-Nextcloud-Talk-Signature", HeaderValue::from_static("sha256=00"));

    let (status, body_json) = response_json(
        handle_nextcloud_talk_webhook(State(configured), headers, Bytes::from("{}")).await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body_json["error"], "Invalid signature");
}

#[tokio::test]
async fn qq_webhook_handles_configuration_validation_and_empty_events() {
    let (status, _) =
        response_json(handle_qq_webhook(State(state()), HeaderMap::new(), Bytes::new()).await)
            .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let configured = state();
    configured.channels.lock().qq =
        Some(Arc::new(QQChannel::new("app-id".into(), "secret".into(), vec!["*".into()])));

    let (status, body) = response_json(
        handle_qq_webhook(State(configured.clone()), HeaderMap::new(), Bytes::from("{}")).await,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "QQ webhook mode not enabled");

    configured.channels.lock().qq_webhook_enabled = true;
    let mut headers = HeaderMap::new();
    headers.insert("X-Bot-Appid", HeaderValue::from_static("other-app"));
    let (status, body) = response_json(
        handle_qq_webhook(State(configured.clone()), headers, Bytes::from("{}")).await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Invalid X-Bot-Appid");

    let (status, body) = response_json(
        handle_qq_webhook(State(configured.clone()), HeaderMap::new(), Bytes::from("{")).await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");

    let validation = r#"{"op":13,"d":{"plain_token":"plain","event_ts":"12345"}}"#;
    let (status, body) = response_json(
        handle_qq_webhook(State(configured.clone()), HeaderMap::new(), Bytes::from(validation))
            .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["plain_token"], "plain");
    assert!(body["signature"].as_str().is_some_and(|value| !value.is_empty()));

    let (status, body) = response_json(
        handle_qq_webhook(State(configured), HeaderMap::new(), Bytes::from(r#"{"op":1}"#)).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn qq_webhook_accepts_matching_app_id_with_whitespace() {
    let configured = state();
    configured.channels.lock().qq =
        Some(Arc::new(QQChannel::new("app-id".into(), "secret".into(), vec!["*".into()])));
    configured.channels.lock().qq_webhook_enabled = true;

    let mut headers = HeaderMap::new();
    headers.insert("X-Bot-Appid", HeaderValue::from_static(" app-id "));

    let (status, body) = response_json(
        handle_qq_webhook(State(configured), headers, Bytes::from(r#"{"op":1}"#)).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn qq_webhook_matching_app_id_does_not_hide_invalid_json() {
    let configured = state();
    configured.channels.lock().qq =
        Some(Arc::new(QQChannel::new("app-id".into(), "secret".into(), vec!["*".into()])));
    configured.channels.lock().qq_webhook_enabled = true;

    let mut headers = HeaderMap::new();
    headers.insert("X-Bot-Appid", HeaderValue::from_static("app-id"));

    let (status, body) =
        response_json(handle_qq_webhook(State(configured), headers, Bytes::from("{")).await).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Invalid JSON payload");
}

#[tokio::test]
async fn channel_reload_saved_credentials_update_existing_webhook_router() {
    use crate::app::agent::config::{SystemConfigStore, schema::ChannelConfigWatcher};
    use crate::global_database::GlobalDatabase;
    use tower::ServiceExt;
    use vw_api_types::id::TenantScope;

    let temp = tempfile::TempDir::new().unwrap();
    let store =
        SystemConfigStore::new(GlobalDatabase::new(temp.path().join("config.sqlite3")).unwrap())
            .unwrap();
    let mut config = Config::default();
    config.config_path = temp.path().join("vibewindow.json");
    let state = state();
    let router = super::super::bootstrap::external_router().with_state(state.clone());
    let source = ChannelConfigWatcher::from_store(store.clone(), &config, false).unwrap();
    let mut watcher =
        tokio::spawn(super::super::channel_reload::watch_gateway_channels_with_source(
            state.clone(),
            config.clone(),
            source,
        ));

    async fn verify(router: &axum::Router, token: &str) -> StatusCode {
        router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!(
                        "/v1/whatsapp?hub.mode=subscribe&hub.verify_token={token}&hub.challenge=ok"
                    ))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }
    assert_eq!(verify(&router, "first").await, StatusCode::NOT_FOUND);
    for token in ["first", "second"] {
        config.channels_config.whatsapp = Some(
            serde_json::from_value(serde_json::json!({
                "access_token":"access", "phone_number_id":"phone", "verify_token":token
            }))
            .unwrap(),
        );
        let mut encrypted = config.clone();
        crate::app::agent::config::schema::encrypt_config_secrets(&mut encrypted).unwrap();
        store.upsert(&TenantScope::default(), &serde_json::to_string(&encrypted).unwrap()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    result = &mut watcher => panic!("watcher exited: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {
                        if verify(&router, token).await == StatusCode::OK { break; }
                    }
                }
            }
        })
        .await
        .unwrap();
    }
    assert_eq!(verify(&router, "first").await, StatusCode::FORBIDDEN);
    config.channels_config.whatsapp = None;
    store.upsert(&TenantScope::default(), &serde_json::to_string(&config).unwrap()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while verify(&router, "second").await != StatusCode::NOT_FOUND {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    watcher.abort();
    let _ = watcher.await;
}

#[tokio::test]
async fn channel_command_cancel_webhook_uses_common_stop_and_help() {
    let state = state();
    let channel = Arc::new(RecordingChannel::default());
    let mut message = channel_message();
    message.channel = "qq".into();
    for command in ["/stop", "/help stop", "/stop extra"] {
        message.content = command.into();
        process_channel_messages(
            &state,
            "QQ",
            &channel,
            &[message.clone()],
            qq_memory_key,
            "QQ",
            true,
        )
        .await;
    }
    let sent = channel.sent.lock();
    assert_eq!(sent.len(), 3);
    assert!(sent[0].content.contains("没有正在执行"));
    assert!(sent[1].content.contains("当前聊天"));
    assert!(sent[2].content.contains("参数错误"));
    assert!(sent.iter().all(
        |reply| reply.recipient == message.reply_target && reply.thread_ts == message.thread_ts
    ));
}

#[tokio::test]
async fn channel_command_cancel_qq_webhook_stops_previous_message_id() {
    let name =
        concat!(module_path!(), "::channel_command_cancel_qq_webhook_stops_previous_message_id");
    let name = name.strip_prefix("vw_agent::").unwrap_or(name);
    if std::env::var("VW_QQ_CANCEL_TEST").as_deref() != Ok(name) {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env("VW_QQ_CANCEL_TEST", name)
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path().join("config"))
            .env("VIBEWINDOW_WORKSPACE", home.path().join("workspace"))
            .output()
            .unwrap();
        assert!(
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    crate::config::update(serde_json::json!({
        "providers": {"channel-cancel-fixture": {"models": {"cancel-model": {"status": "active"}}}}
    }))
    .await
    .unwrap();
    let tmp = tempfile::tempdir().unwrap();
    crate::project::from_directory(&crate::session::ui_store::current_tenant_scope(), tmp.path())
        .await
        .unwrap();
    let replies = Arc::new(tokio::sync::Mutex::new(Vec::<SendMessage>::new()));
    let mut state = state();
    state.model = "cancel-model".into();
    {
        let mut config = state.config.lock();
        config.workspace_dir = tmp.path().to_path_buf();
        config.default_provider = Some("channel-cancel-fixture".into());
    }
    let qq = Arc::new(
        QQChannel::new(
            "fixture-app".into(),
            "fixture-secret".into(),
            vec!["allowed-sender".into()],
        )
        .with_outbound_recorder(replies.clone()),
    );
    state.channels.lock().qq = Some(qq);
    state.channels.lock().qq_webhook_enabled = true;
    let payload = |id: &str, content: &str, sender: &str| {
        Bytes::from(serde_json::to_vec(&serde_json::json!({
        "op":0,"t":"C2C_MESSAGE_CREATE","d":{"id":id,"content":content,"author":{"user_openid":sender}}
    })).unwrap())
    };
    let mut headers = HeaderMap::new();
    headers.insert("X-Bot-Appid", HeaderValue::from_static("fixture-app"));
    assert_eq!(
        handle_qq_webhook(
            State(state.clone()),
            headers.clone(),
            payload("new-id", "/new", "allowed-sender")
        )
        .await
        .status(),
        StatusCode::OK
    );
    let runtime = super::super::channel_runtime::channel_runtime(&state).unwrap();
    let mut msg = channel_message();
    msg.channel = "qq".into();
    msg.sender = "allowed-sender".into();
    msg.reply_target = "user:allowed-sender".into();
    msg.thread_ts = Some("new-id".into());
    let snapshot = crate::channels::resolve_channel_message_snapshot(&runtime, &msg).await.unwrap();
    let session = crate::channels::sender_session_store()
        .lock()
        .unwrap()
        .get(&snapshot.scope)
        .unwrap()
        .clone();
    let stream = Arc::new(crate::session::processor::ChannelCancelStream::default());
    crate::session::processor::register_channel_cancel_stream(&session, stream.clone());
    let running = tokio::spawn(handle_qq_webhook(
        State(state.clone()),
        headers.clone(),
        payload("ordinary-id", "Tell a story", "allowed-sender"),
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !stream.started.load(std::sync::atomic::Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    // Actor allowlisting still runs before cancellation control.
    handle_qq_webhook(
        State(state.clone()),
        headers.clone(),
        payload("denied-stop", "/stop", "other-sender"),
    )
    .await;
    assert!(!stream.exited.load(std::sync::atomic::Ordering::Acquire));
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        handle_qq_webhook(
            State(state.clone()),
            headers,
            payload("stop-id", "/stop", "allowed-sender"),
        ),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(stream.exited.load(std::sync::atomic::Ordering::Acquire));
    running.await.unwrap();
    let sent = replies.lock().await;
    assert!(
        sent.iter().any(|reply| reply.content.contains("执行已停止")
            && reply.thread_ts.as_deref() == Some("stop-id")
            && reply.recipient == "user:allowed-sender"),
        "captured QQ replies: {sent:?}"
    );
    assert!(!sent.iter().any(|reply| reply.content.contains("completed naturally")));
}

#[tokio::test]
async fn channel_command_cancel_webhook_failed_send_preserves_state() {
    let state = state();
    let channel = Arc::new(RecordingChannel { fail_send: true, ..Default::default() });
    let mut msg = channel_message();
    msg.content = "/stop".into();
    process_channel_messages(
        &state,
        "recording",
        &channel,
        &[msg],
        qq_memory_key,
        "recording",
        true,
    )
    .await;
    assert_eq!(channel.sent.lock().len(), 1);
    let runtime = super::super::channel_runtime::channel_runtime(&state).unwrap();
    assert!(runtime.conversation_histories.lock().unwrap().is_empty());
    assert!(runtime.project_state.lock().unwrap().active.is_empty());
    assert!(!runtime.session_full_access);
}
