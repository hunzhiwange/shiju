use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::providers::traits::{StreamChunk, StreamError, StreamResult};
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::{Body, Bytes, to_bytes};
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::{StreamExt, stream};
use parking_lot::Mutex;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
struct RecordedChatCall {
    messages: Vec<ChatMessage>,
    model: String,
    temperature: f64,
}

#[derive(Debug, Clone)]
enum StreamItem {
    Delta(&'static str),
    EmptyDelta,
    Final,
    Error(&'static str),
}

struct TestProvider {
    chat_result: Result<String, String>,
    supports_streaming: bool,
    stream_items: Vec<StreamItem>,
    chat_calls: Mutex<Vec<RecordedChatCall>>,
    stream_calls: Mutex<Vec<RecordedChatCall>>,
}

impl TestProvider {
    fn chat_ok(text: impl Into<String>) -> Self {
        Self {
            chat_result: Ok(text.into()),
            supports_streaming: false,
            stream_items: Vec::new(),
            chat_calls: Mutex::new(Vec::new()),
            stream_calls: Mutex::new(Vec::new()),
        }
    }

    fn chat_err(message: impl Into<String>) -> Self {
        Self {
            chat_result: Err(message.into()),
            supports_streaming: false,
            stream_items: Vec::new(),
            chat_calls: Mutex::new(Vec::new()),
            stream_calls: Mutex::new(Vec::new()),
        }
    }

    fn streaming(items: Vec<StreamItem>) -> Self {
        Self {
            chat_result: Ok("unused".to_string()),
            supports_streaming: true,
            stream_items: items,
            chat_calls: Mutex::new(Vec::new()),
            stream_calls: Mutex::new(Vec::new()),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for TestProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.chat_with_history(&[ChatMessage::user(message)], model, temperature).await
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.chat_calls.lock().push(RecordedChatCall {
            messages: messages.to_vec(),
            model: model.to_string(),
            temperature,
        });

        self.chat_result.clone().map_err(|message| anyhow::anyhow!(message))
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
        _options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        self.stream_calls.lock().push(RecordedChatCall {
            messages: messages.to_vec(),
            model: model.to_string(),
            temperature,
        });

        let chunks = self.stream_items.clone().into_iter().map(|item| match item {
            StreamItem::Delta(delta) => Ok(StreamChunk::delta(delta)),
            StreamItem::EmptyDelta => Ok(StreamChunk::delta("")),
            StreamItem::Final => Ok(StreamChunk::final_chunk()),
            StreamItem::Error(message) => Err(StreamError::Provider(message.to_string())),
        });
        stream::iter(chunks).boxed()
    }
}

fn state_with_provider(
    provider: Arc<TestProvider>,
    require_pairing: bool,
    paired_tokens: &[&str],
    webhook_rate_limit: u32,
) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    let mut config = Config::default();
    config.default_provider = Some("test-provider".to_string());
    let paired_tokens = paired_tokens.iter().map(|token| token.to_string()).collect::<Vec<_>>();

    AppState {
        config: Arc::new(Mutex::new(config)),
        provider,
        model: "state-model".to_string(),
        temperature: 0.25,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, &paired_tokens)),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, webhook_rate_limit, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn peer_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 42_000)
}

async fn chat_response(state: AppState, headers: HeaderMap, body: impl Into<Bytes>) -> Response {
    handle_v1_chat_completions(State(state), ConnectInfo(peer_addr()), headers, body.into())
        .await
        .into_response()
}

async fn response_json(response: Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    let payload = serde_json::from_slice(&bytes).expect("response should be json");
    (status, payload)
}

async fn response_text(response: Response) -> (StatusCode, HeaderMap, String) {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    let text = String::from_utf8(bytes.to_vec()).expect("response should be utf8");
    (status, headers, text)
}

fn bearer(token: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static(token));
    headers
}

#[tokio::test]
async fn handle_v1_chat_completions_rejects_rate_limited_request() {
    let provider = Arc::new(TestProvider::chat_ok("unused"));
    let state = state_with_provider(provider.clone(), false, &[], 1);
    let body = br#"{"messages":[{"role":"user","content":"hello"}]}"#;

    let first = chat_response(state.clone(), HeaderMap::new(), Bytes::from_static(body)).await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = chat_response(state, HeaderMap::new(), Bytes::from_static(body)).await;
    let (status, payload) = response_json(second).await;

    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(payload["error"]["code"], "rate_limit_exceeded");
    assert_eq!(provider.chat_calls.lock().len(), 1);
}

#[tokio::test]
async fn handle_v1_chat_completions_rejects_missing_pairing_bearer() {
    let state =
        state_with_provider(Arc::new(TestProvider::chat_ok("unused")), true, &["token"], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload["error"]["code"], "invalid_api_key");
}

#[tokio::test]
async fn handle_v1_chat_completions_rejects_invalid_json_body() {
    let state = state_with_provider(Arc::new(TestProvider::chat_ok("unused")), false, &[], 100);

    let response = chat_response(state, HeaderMap::new(), Bytes::from_static(b"{")).await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"]["code"], "invalid_json");
}

#[tokio::test]
async fn handle_v1_chat_completions_rejects_empty_messages() {
    let state = state_with_provider(Arc::new(TestProvider::chat_ok("unused")), false, &[], 100);

    let response =
        chat_response(state, HeaderMap::new(), Bytes::from_static(br#"{"messages":[]}"#)).await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload["error"]["code"], "invalid_messages");
}

#[tokio::test]
async fn handle_v1_chat_completions_rejects_oversized_body() {
    let state = state_with_provider(Arc::new(TestProvider::chat_ok("unused")), false, &[], 100);
    let body = Bytes::from(vec![b'x'; CHAT_COMPLETIONS_MAX_BODY_SIZE + 1]);

    let response = chat_response(state, HeaderMap::new(), body).await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(payload["error"]["code"], "request_too_large");
}

#[tokio::test]
async fn handle_v1_chat_completions_returns_non_streaming_completion() {
    let provider = Arc::new(TestProvider::chat_ok("assistant text"));
    let state = state_with_provider(provider.clone(), true, &["paired-token"], 100);

    let response = chat_response(
        state,
        bearer("Bearer paired-token"),
        Bytes::from_static(
            br#"{
                "model": "request-model",
                "messages": [
                    {"role": "system", "content": "guide"},
                    {"role": "user", "content": "hello"}
                ],
                "temperature": 0.5,
                "stream": false
            }"#,
        ),
    )
    .await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["object"], "chat.completion");
    assert_eq!(payload["model"], "request-model");
    assert_eq!(payload["choices"][0]["message"]["role"], "assistant");
    assert_eq!(payload["choices"][0]["message"]["content"], "assistant text");
    assert_eq!(payload["choices"][0]["finish_reason"], "stop");
    assert_eq!(payload["usage"]["prompt_tokens"], 2);
    assert_eq!(payload["usage"]["completion_tokens"], 3);
    assert_eq!(payload["usage"]["total_tokens"], 5);

    let calls = provider.chat_calls.lock();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].model, "request-model");
    assert_eq!(calls[0].temperature, 0.5);
    assert_eq!(calls[0].messages[0].role, "system");
    assert_eq!(calls[0].messages[1].content, "hello");
}

#[tokio::test]
async fn handle_v1_chat_completions_uses_state_defaults_for_empty_model_and_temperature() {
    let provider = Arc::new(TestProvider::chat_ok("ok"));
    let state = state_with_provider(provider.clone(), false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"model":"","messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["model"], "state-model");
    let calls = provider.chat_calls.lock();
    assert_eq!(calls[0].model, "state-model");
    assert_eq!(calls[0].temperature, 0.25);
}

#[tokio::test]
async fn handle_v1_chat_completions_uses_unknown_provider_label_when_config_has_no_default() {
    let provider = Arc::new(TestProvider::chat_ok("ok"));
    let state = state_with_provider(provider, false, &[], 100);
    state.config.lock().default_provider = None;

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn handle_v1_chat_completions_returns_provider_error_without_leaking_details() {
    let provider = Arc::new(TestProvider::chat_err("secret provider token leaked"));
    let state = state_with_provider(provider, false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(payload["error"]["message"], "LLM request failed");
    assert_eq!(payload["error"]["code"], "provider_error");
}

#[tokio::test]
async fn handle_v1_chat_completions_streams_single_chunk_when_provider_lacks_streaming() {
    let provider = Arc::new(TestProvider::chat_ok("fallback text"));
    let state = state_with_provider(provider, false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"stream":true,"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, headers, text) = response_text(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "text/event-stream");
    assert!(text.contains("\"role\":\"assistant\""));
    assert!(text.contains("\"content\":\"fallback text\""));
    assert!(text.contains("\"finish_reason\":\"stop\""));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn handle_v1_chat_completions_streams_sanitized_fallback_error() {
    let provider = Arc::new(TestProvider::chat_err("raw provider failure"));
    let state = state_with_provider(provider, false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"stream":true,"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, _, text) = response_text(response).await;

    assert_eq!(status, StatusCode::OK);
    assert!(text.contains("LLM request failed: raw provider failure"));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn handle_v1_chat_completions_streams_native_chunks_and_done() {
    let provider = Arc::new(TestProvider::streaming(vec![
        StreamItem::Delta("hel"),
        StreamItem::EmptyDelta,
        StreamItem::Delta("lo"),
        StreamItem::Final,
    ]));
    let state = state_with_provider(provider.clone(), false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"stream":true,"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, headers, text) = response_text(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
    assert_eq!(headers[header::CONNECTION], "keep-alive");
    assert!(text.contains("\"role\":\"assistant\""));
    assert!(text.contains("\"content\":\"hel\""));
    assert!(text.contains("\"content\":\"lo\""));
    assert!(text.contains("data: [DONE]"));
    assert_eq!(provider.stream_calls.lock().len(), 1);
}

#[tokio::test]
async fn handle_v1_chat_completions_streams_native_error_chunk() {
    let provider = Arc::new(TestProvider::streaming(vec![StreamItem::Error("stream failed")]));
    let state = state_with_provider(provider, false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"stream":true,"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, _, text) = response_text(response).await;

    assert_eq!(status, StatusCode::OK);
    assert!(text.contains("LLM request failed: Provider error: stream failed"));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn handle_v1_chat_completions_streams_done_after_native_error_without_success_event() {
    let provider = Arc::new(TestProvider::streaming(vec![
        StreamItem::Error("stream failed"),
        StreamItem::Final,
    ]));
    let state = state_with_provider(provider, false, &[], 100);

    let response = chat_response(
        state,
        HeaderMap::new(),
        Bytes::from_static(br#"{"stream":true,"messages":[{"role":"user","content":"hello"}]}"#),
    )
    .await;
    let (status, _, text) = response_text(response).await;

    assert_eq!(status, StatusCode::OK);
    assert!(text.contains("LLM request failed: Provider error: stream failed"));
    assert!(text.ends_with("data: [DONE]\n\n"));
}

#[tokio::test]
async fn handle_v1_models_rejects_missing_pairing_bearer() {
    let state =
        state_with_provider(Arc::new(TestProvider::chat_ok("unused")), true, &["token"], 100);

    let response = handle_v1_models(State(state), HeaderMap::new()).await.into_response();
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload["error"]["code"], "invalid_api_key");
}

#[tokio::test]
async fn handle_v1_models_allows_request_when_pairing_is_disabled() {
    let state = state_with_provider(Arc::new(TestProvider::chat_ok("unused")), false, &[], 100);

    let response = handle_v1_models(State(state), HeaderMap::new()).await.into_response();
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"][0]["id"], "state-model");
}

#[tokio::test]
async fn handle_v1_models_returns_configured_model_for_valid_request() {
    let state =
        state_with_provider(Arc::new(TestProvider::chat_ok("unused")), true, &["token"], 100);

    let response = handle_v1_models(State(state), bearer("Bearer token")).await.into_response();
    let (status, payload) = response_json(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["object"], "list");
    assert_eq!(payload["data"][0]["id"], "state-model");
    assert_eq!(payload["data"][0]["object"], "model");
    assert_eq!(payload["data"][0]["owned_by"], "vibewindow");
}

#[test]
fn chunk_bytes_without_done_omits_done_marker_and_empty_fields() {
    let bytes = chunk_bytes(
        "chatcmpl-test".to_string(),
        1_234_567_890,
        "model".to_string(),
        None,
        None,
        None,
        false,
    );
    let text = String::from_utf8(bytes.to_vec()).expect("chunk should be utf8");

    assert!(text.starts_with("data: "));
    assert!(!text.contains("data: [DONE]"));
    assert!(!text.contains("\"role\""));
    assert!(!text.contains("\"content\""));
}

#[test]
fn helper_record_functions_accept_success_and_failure_events() {
    let state = state_with_provider(Arc::new(TestProvider::chat_ok("unused")), false, &[], 100);

    record_success(&state, "provider", "model", Duration::from_millis(5));
    record_failure(&state, "provider", "model", Duration::from_millis(5), "failed");
}

#[tokio::test]
async fn response_helpers_read_empty_body_text() {
    let response = Response::new(Body::empty());

    let (status, _, text) = response_text(response).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(text, "");
}
