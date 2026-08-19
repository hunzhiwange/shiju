use super::{extract_bearer_token, require_auth};
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

fn state(require_pairing: bool, paired_tokens: &[String]) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, paired_tokens)),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
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
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {token}");
    headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&value).unwrap());
    headers
}

#[test]
fn extract_bearer_token_requires_bearer_prefix() {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer token-a"));

    assert_eq!(extract_bearer_token(&headers), Some("token-a"));

    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Basic token-a"));
    assert_eq!(extract_bearer_token(&headers), None);
}

#[test]
fn extract_bearer_token_rejects_missing_invalid_and_empty_values() {
    assert_eq!(extract_bearer_token(&HeaderMap::new()), None);

    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_bytes(b"\xff").unwrap());
    assert_eq!(extract_bearer_token(&headers), None);

    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));
    assert_eq!(extract_bearer_token(&headers), None);
}

#[test]
fn require_auth_allows_everything_when_pairing_is_disabled() {
    let state = state(false, &[]);

    assert!(require_auth(&state, &HeaderMap::new()).is_ok());
    assert!(require_auth(&state, &bearer_headers("ignored")).is_ok());
}

#[test]
fn require_auth_accepts_only_paired_bearer_token_when_pairing_is_enabled() {
    let token = "paired-token".to_string();
    let state = state(true, std::slice::from_ref(&token));

    assert!(require_auth(&state, &bearer_headers(&token)).is_ok());

    for headers in [
        HeaderMap::new(),
        bearer_headers("wrong-token"),
        {
            let mut headers = HeaderMap::new();
            headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Token paired-token"));
            headers
        },
        {
            let mut headers = HeaderMap::new();
            headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));
            headers
        },
    ] {
        let (status, body) = require_auth(&state, &headers).unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.0["error"].as_str().unwrap().contains("Unauthorized"));
    }
}
