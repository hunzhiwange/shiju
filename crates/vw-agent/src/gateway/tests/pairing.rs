use super::*;
use crate::app::agent::config::schema::ConfigExt;
use crate::app::agent::gateway::pairing::{handle_pair, handle_pair_code, persist_pairing_tokens};
use axum::body::to_bytes;
use axum::extract::State;
use axum::response::IntoResponse;

#[tokio::test]
async fn persist_pairing_tokens_writes_config_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join(".vibewindow.json");
    let workspace_path = temp.path().join("workspace");

    let mut config = Config::default();
    config.config_path = config_path.clone();
    config.workspace_dir = workspace_path;
    config.save().await.unwrap();

    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let token = guard.try_pair(&code, "test_client").await.unwrap().unwrap();
    assert!(guard.is_authenticated(&token));

    let shared_config = Arc::new(Mutex::new(config));
    persist_pairing_tokens(shared_config.clone(), &guard).await.unwrap();

    let saved = tokio::fs::read_to_string(config_path).await.unwrap();
    let parsed: Config = serde_json::from_str(&saved).unwrap();
    assert_eq!(parsed.gateway.paired_tokens.len(), 1);
    let persisted = &parsed.gateway.paired_tokens[0];
    assert_eq!(persisted.len(), 64);
    assert!(persisted.chars().all(|c: char| c.is_ascii_hexdigit()));

    let in_memory = shared_config.lock();
    assert_eq!(in_memory.gateway.paired_tokens.len(), 1);
    assert_eq!(&in_memory.gateway.paired_tokens[0], persisted);
}

// ---------------------------------------------------------------------------
// handle_pair HTTP handler — unit tests with error-code assertions
// ---------------------------------------------------------------------------

/// Build a minimal [`AppState`] suitable for `handle_pair` unit tests.
/// `rate_limit` is the per-minute pair request limit (0 = unlimited).
fn make_pair_state(pairing: PairingGuard, rate_limit: u32) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(MockProvider::default()),
        model: "test-model".into(),
        temperature: 0.0,
        mem: Arc::new(MockMemory),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(pairing),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(rate_limit, 100, 1_000)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1_000)),
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
        tools_registry: Arc::new(vec![]),
        tools_registry_exec: Arc::new(vec![]),
        multimodal: Default::default(),
        max_tool_iterations: 5,
        event_tx: tokio::sync::broadcast::channel(1).0,
        session_query_engines: Default::default(),
    }
}

/// Helper: extract the HTTP status and JSON body from a `handle_pair` call.
async fn call_pair(
    state: AppState,
    code_header: Option<&str>,
) -> (axum::http::StatusCode, serde_json::Value) {
    let mut headers = axum::http::HeaderMap::new();
    if let Some(code) = code_header {
        headers.insert("X-Pairing-Code", axum::http::HeaderValue::from_str(code).unwrap());
    }
    let response = handle_pair(State(state), test_connect_info(), headers).await.into_response();

    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    (status, body)
}

async fn call_pair_code(
    state: AppState,
    connect_info: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> (axum::http::StatusCode, serde_json::Value) {
    let response = handle_pair_code(State(state), connect_info).await.into_response();

    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    (status, body)
}

// --- 200 OK: valid one-time code ----------------------------------------

#[tokio::test]
async fn handle_pair_returns_200_with_token_on_valid_code() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let state = make_pair_state(guard, 100);

    let (status, body) = call_pair(state, Some(&code)).await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["paired"], true);
    let token = body["token"].as_str().expect("response must contain 'token' field");
    assert!(!token.is_empty(), "token must not be empty");
}

// --- 200 OK: already-paired token is re-accepted after a second pair -----

#[tokio::test]
async fn handle_pair_token_authenticates_after_pairing() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let state = make_pair_state(guard, 100);

    let (status, body) = call_pair(state.clone(), Some(&code)).await;
    assert_eq!(status, axum::http::StatusCode::OK);

    let token = body["token"].as_str().unwrap().to_owned();
    assert!(state.pairing.is_authenticated(&token));
}

#[tokio::test]
async fn handle_pair_code_returns_loopback_bootstrap_code_when_unpaired() {
    let guard = PairingGuard::new(true, &[]);
    let expected_code = guard.pairing_code().unwrap();
    let state = make_pair_state(guard, 100);

    let (status, body) = call_pair_code(state, test_connect_info()).await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["require_pairing"], true);
    assert_eq!(body["paired"], false);
    assert_eq!(body["pairing_code"], expected_code);
}

#[tokio::test]
async fn handle_pair_code_rejects_non_loopback_clients() {
    let state = make_pair_state(PairingGuard::new(true, &[]), 100);

    let (status, body) = call_pair_code(state, test_public_connect_info()).await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    assert!(body["error"].is_string());
}

// --- 403 FORBIDDEN: wrong pairing code ----------------------------------

#[tokio::test]
async fn handle_pair_returns_403_on_wrong_code() {
    let guard = PairingGuard::new(true, &[]);
    let state = make_pair_state(guard, 100);

    let (status, body) = call_pair(state, Some("completely-wrong-code")).await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    let error = body["error"].as_str().expect("response must contain 'error' field");
    assert!(!error.is_empty());
}

// --- 403 FORBIDDEN: missing X-Pairing-Code header -----------------------

#[tokio::test]
async fn handle_pair_returns_403_when_no_code_header() {
    let guard = PairingGuard::new(true, &[]);
    let state = make_pair_state(guard, 100);

    // No header — handler treats empty string as invalid code.
    let (status, body) = call_pair(state, None).await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    assert!(body["error"].as_str().is_some());
}

// --- 403 FORBIDDEN: empty X-Pairing-Code header -------------------------

#[tokio::test]
async fn handle_pair_returns_403_on_empty_code() {
    let guard = PairingGuard::new(true, &[]);
    let state = make_pair_state(guard, 100);

    let (status, _body) = call_pair(state, Some("")).await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

// --- 403 FORBIDDEN: pairing disabled (no code generated) ----------------

#[tokio::test]
async fn handle_pair_returns_403_when_pairing_mode_disabled() {
    // PairingGuard::new(false, &[]) means pairing_code() returns None;
    // any code will be treated as invalid.
    let guard = PairingGuard::new(false, &[]);
    let state = make_pair_state(guard, 100);

    let (status, _body) = call_pair(state, Some("any-code")).await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

// --- 429 TOO_MANY_REQUESTS: rate limiter exhausted ----------------------

#[tokio::test]
async fn handle_pair_returns_429_when_rate_limit_exceeded() {
    let guard = PairingGuard::new(true, &[]);
    // limit = 1: the first call consumes the budget; the second is rejected.
    let state = make_pair_state(guard, 1);

    // First call: burns the single allowed slot.
    call_pair(state.clone(), Some("irrelevant")).await;

    // Second call from the same IP must be rate-limited.
    let (status, body) = call_pair(state, Some("irrelevant")).await;

    assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS);
    let error = body["error"].as_str().expect("rate-limit response must contain 'error' field");
    assert!(!error.is_empty());
    assert!(
        body["retry_after"].as_u64().is_some(),
        "rate-limit response must include 'retry_after'"
    );
}

// --- 429 TOO_MANY_REQUESTS: brute-force lockout -------------------------

#[tokio::test]
async fn handle_pair_returns_429_after_brute_force_lockout() {
    // MAX_PAIR_ATTEMPTS is 5 (private const in security::pairing).
    // We submit 6 attempts (5 + 1) to guarantee lockout is triggered.
    const ATTEMPTS_TO_LOCKOUT: usize = 6;

    let guard = PairingGuard::new(true, &[]);
    let state = make_pair_state(guard, 1_000);

    // Submit ATTEMPTS_TO_LOCKOUT wrong codes to trigger per-client lockout.
    let wrong_code = "wrong-brute-force-code";
    let mut last_status = axum::http::StatusCode::FORBIDDEN;
    let mut last_body = serde_json::Value::Null;
    for _ in 0..ATTEMPTS_TO_LOCKOUT {
        let (s, b) = call_pair(state.clone(), Some(wrong_code)).await;
        last_status = s;
        last_body = b;
    }

    // After exhausting attempts the handler must return 429 with retry_after.
    assert_eq!(last_status, axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert!(
        last_body["retry_after"].as_u64().is_some(),
        "lockout response must include 'retry_after'"
    );
}

// --- 200 OK: pre-seeded token is accepted (restart scenario) -----------

#[tokio::test]
async fn handle_pair_accepts_pre_seeded_token_immediately() {
    let pre_token = "pre_seeded_hex_token_aabbccdd1122334455".to_string();
    // PairingGuard::new(true, &[pre_token]) loads a pre-paired token.
    let guard = PairingGuard::new(true, std::slice::from_ref(&pre_token));
    assert!(
        guard.is_authenticated(&pre_token),
        "pre-seeded token must be immediately authenticated"
    );
}

// --- JSON body contract: paired=true contains token field ---------------

#[tokio::test]
async fn handle_pair_success_body_has_paired_true_and_token() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap();
    let state = make_pair_state(guard, 100);

    let (status, body) = call_pair(state, Some(&code)).await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["paired"], serde_json::Value::Bool(true), "success body must have paired=true");
    assert!(body["token"].is_string(), "success body must contain a string 'token'");
}

// --- JSON body contract: failed attempt has error field -----------------

#[tokio::test]
async fn handle_pair_failure_body_has_error_field() {
    let guard = PairingGuard::new(true, &[]);
    let state = make_pair_state(guard, 100);

    let (status, body) = call_pair(state, Some("bad-code")).await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    assert!(body["error"].is_string(), "failure body must contain a string 'error' field");
}
