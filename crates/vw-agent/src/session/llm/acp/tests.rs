//! 验证 ACP LLM 适配器的路由、能力和公共调用契约。

use std::sync::Arc;

use super::*;
use crate::app::agent::session::message;

fn unknown_error(message: &str) -> Error {
    Error::Api(message::AssistantError::Unknown { message: message.to_string() })
}

fn api_error(message: &str) -> Error {
    Error::Api(message::AssistantError::APIError {
        message: message.to_string(),
        status_code: Some(409),
        is_retryable: false,
        response_headers: None,
        response_body: None,
        metadata: None,
    })
}

fn cached_client(
    client: Arc<AcpClient>,
    output_tx: Option<mpsc::UnboundedSender<AcpJsonRpcMessage>>,
) -> Arc<CachedAcpClient> {
    Arc::new(CachedAcpClient {
        client,
        prompt_lock: Arc::new(tokio::sync::Mutex::new(())),
        output_tx: Arc::new(Mutex::new(output_tx)),
    })
}

#[test]
fn to_api_error_wraps_display_text_with_acp_prefix() {
    let err = to_api_error("agent failed");

    match err {
        Error::Api(message::AssistantError::Unknown { message }) => {
            assert_eq!(message, "acp: agent failed");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn acp_session_changed_message_detection_is_case_insensitive_and_requires_exact_marker() {
    assert!(is_acp_session_changed_message("ACP SESSION CHANGED: old -> new"));
    assert!(is_acp_session_changed_message("prefix acp session changed: details"));
    assert!(!is_acp_session_changed_message("acp session changed"));
    assert!(!is_acp_session_changed_message("acp session changed : details"));
    assert!(!is_acp_session_changed_message("session changed without acp marker"));
}

#[test]
fn acp_option_error_formats_as_api_unknown_error() {
    let err = acp_option_error("bad value");

    match err {
        Error::Api(message::AssistantError::Unknown { message }) => {
            assert_eq!(message, "acp option error: bad value");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn is_acp_session_changed_error_checks_only_known_api_error_variants() {
    assert!(is_acp_session_changed_error(&unknown_error("acp session changed: one")));
    assert!(is_acp_session_changed_error(&api_error("acp session changed: two")));
    assert!(!is_acp_session_changed_error(&unknown_error("other error")));
    assert!(!is_acp_session_changed_error(&api_error("other error")));
    assert!(!is_acp_session_changed_error(&Error::Aborted));
}

#[test]
fn parsed_acp_options_default_keeps_every_optional_field_empty() {
    let parsed = ParsedAcpOptions::default();

    assert_eq!(parsed.permission_mode, None);
    assert_eq!(parsed.non_interactive_permissions, None);
    assert_eq!(parsed.auth_policy, None);
    assert_eq!(parsed.session_mode, None);
    assert_eq!(parsed.session_options, None);
    assert!(parsed.session_config_options.is_empty());
}

#[test]
fn acp_replay_strategy_values_are_distinct() {
    assert_ne!(AcpReplayStrategy::Discard, AcpReplayStrategy::Full);
    assert_ne!(AcpReplayStrategy::Summary, AcpReplayStrategy::Recent);
}

#[test]
fn cached_acp_client_stores_client_lock_and_forwards_output() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let client = Arc::new(AcpClient::new(
        "unit",
        vw_acp::AcpAgentConfig {
            command: "unit-agent".to_string(),
            args: Vec::new(),
            env: Default::default(),
        },
    ));
    let cached = cached_client(client.clone(), Some(tx));

    assert!(Arc::ptr_eq(&cached.client, &client));
    assert!(cached.prompt_lock.try_lock().is_ok());
    assert!(cached.output_tx.lock().as_ref().is_some());

    let message: AcpJsonRpcMessage = serde_json::from_value(serde_json::json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": { "text": "hello" }
    }))
    .expect("json-rpc notification should deserialize");
    cached
        .output_tx
        .lock()
        .as_ref()
        .expect("sender should be present")
        .send(message)
        .expect("message should send");
    assert!(rx.try_recv().is_ok());
}

#[test]
fn cached_acp_client_entry_keeps_key_and_shared_client() {
    let client = Arc::new(AcpClient::new(
        "unit",
        vw_acp::AcpAgentConfig {
            command: "unit-agent".to_string(),
            args: Vec::new(),
            env: Default::default(),
        },
    ));
    let cached = cached_client(client, None);
    let entry = CachedAcpClientEntry { key: "unit-key".to_string(), client: cached.clone() };

    assert_eq!(entry.key, "unit-key");
    assert!(Arc::ptr_eq(&entry.client, &cached));
}
