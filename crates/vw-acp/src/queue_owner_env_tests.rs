//! 队列所有者环境载荷解析的单元测试。

use std::sync::{Mutex, MutexGuard};

use crate::{
    AuthPolicy, NonInteractivePermissionPolicy, PermissionMode, QUEUE_OWNER_PAYLOAD_ENV,
    QueueOwnerPayloadError, parse_queue_owner_payload, queue_owner_runtime_options_from_env,
    run_queue_owner_from_env,
};

static QUEUE_OWNER_ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn queue_owner_env_test_lock() -> MutexGuard<'static, ()> {
    QUEUE_OWNER_ENV_TEST_LOCK.lock().expect("queue owner env test lock should acquire")
}

fn minimal_payload(permission_mode: &str) -> String {
    format!(
        r#"{{
            "sessionId": "session-1",
            "permissionMode": "{permission_mode}"
        }}"#
    )
}

#[test]
fn parse_queue_owner_payload_maps_supported_fields() {
    let options = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "approve-reads",
            "mcpServers": [
                {
                    "name": "stdio-server",
                    "command": "/bin/echo"
                }
            ],
            "nonInteractivePermissions": "fail",
            "authCredentials": {
                "token": "secret",
                "numeric": 1
            },
            "authPolicy": "fail",
            "suppressSdkConsoleErrors": true,
            "verbose": true,
            "ttlMs": 300000,
            "maxQueueDepth": 1.2,
            "promptRetries": 2.6
        }"#,
    )
    .expect("payload should parse");

    assert_eq!(options.session_id, "session-1");
    assert_eq!(options.permission_mode, PermissionMode::ApproveReads);
    assert_eq!(options.mcp_servers.as_ref().map(Vec::len), Some(1));
    assert_eq!(options.non_interactive_permissions, Some(NonInteractivePermissionPolicy::Fail));
    assert_eq!(
        options
            .auth_credentials
            .as_ref()
            .and_then(|entries: &std::collections::HashMap<String, String>| entries.get("token")),
        Some(&"secret".to_string())
    );
    assert_eq!(
        options
            .auth_credentials
            .as_ref()
            .map(|entries: &std::collections::HashMap<String, String>| entries.len()),
        Some(1)
    );
    assert_eq!(options.auth_policy, Some(AuthPolicy::Fail));
    assert_eq!(options.suppress_sdk_console_errors, Some(true));
    assert_eq!(options.verbose, Some(true));
    assert_eq!(options.ttl_ms, Some(300000));
    assert_eq!(options.max_queue_depth, Some(1));
    assert_eq!(options.prompt_retries, Some(3));
}

#[test]
fn parse_queue_owner_payload_maps_remaining_supported_modes_and_optional_values() {
    let approve_all = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-approve-all",
            "permissionMode": "approve-all",
            "mcpServers": null,
            "nonInteractivePermissions": "deny",
            "authCredentials": {
                "token": "secret"
            },
            "authPolicy": "skip",
            "suppressSdkConsoleErrors": false,
            "verbose": false,
            "ttlMs": 0,
            "maxQueueDepth": 0.2,
            "promptRetries": -1.4
        }"#,
    )
    .expect("approve-all payload should parse");

    assert_eq!(approve_all.permission_mode, PermissionMode::ApproveAll);
    assert_eq!(approve_all.mcp_servers, None);
    assert_eq!(approve_all.non_interactive_permissions, Some(NonInteractivePermissionPolicy::Deny));
    assert_eq!(approve_all.auth_policy, Some(AuthPolicy::Skip));
    assert_eq!(approve_all.suppress_sdk_console_errors, Some(false));
    assert_eq!(approve_all.verbose, Some(false));
    assert_eq!(approve_all.ttl_ms, Some(0));
    assert_eq!(approve_all.max_queue_depth, Some(1));
    assert_eq!(approve_all.prompt_retries, Some(0));

    let deny_all = parse_queue_owner_payload(&minimal_payload("deny-all"))
        .expect("deny-all payload should parse");

    assert_eq!(deny_all.permission_mode, PermissionMode::DenyAll);
}

#[test]
fn parse_queue_owner_payload_maps_empty_optional_collections() {
    let options = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "approve-all",
            "mcpServers": [],
            "authCredentials": {
                "numeric": 1,
                "flag": true
            }
        }"#,
    )
    .expect("empty optional collections should parse");

    assert_eq!(options.mcp_servers.as_ref().map(Vec::len), Some(0));
    assert_eq!(
        options
            .auth_credentials
            .as_ref()
            .map(|entries: &std::collections::HashMap<String, String>| entries.len()),
        Some(0)
    );
}

#[test]
fn parse_queue_owner_payload_clamps_large_retry_and_depth_values() {
    let options = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "deny-all",
            "ttlMs": 340282366920938463463374607431768211455,
            "maxQueueDepth": 340282366920938463463374607431768211455,
            "promptRetries": 340282366920938463463374607431768211455
        }"#,
    )
    .expect("large numeric optional values should parse");

    assert_eq!(options.ttl_ms, None);
    assert_eq!(options.max_queue_depth, Some(usize::MAX));
    assert_eq!(options.prompt_retries, Some(u64::MAX));
}

#[test]
fn parse_queue_owner_payload_drops_negative_ttl_and_keeps_plain_integer_optionals() {
    let options = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "deny-all",
            "ttlMs": -1,
            "maxQueueDepth": 5,
            "promptRetries": 4
        }"#,
    )
    .expect("negative ttl with plain integer optionals should parse");

    assert_eq!(options.ttl_ms, None);
    assert_eq!(options.max_queue_depth, Some(5));
    assert_eq!(options.prompt_retries, Some(4));
}

#[test]
fn parse_queue_owner_payload_rounds_fractional_depth_and_retries_to_nearest() {
    let options = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "deny-all",
            "maxQueueDepth": 3.5,
            "promptRetries": 2.4
        }"#,
    )
    .expect("fractional depth and retries should round to nearest");

    assert_eq!(options.max_queue_depth, Some(4));
    assert_eq!(options.prompt_retries, Some(2));
}

#[test]
fn parse_queue_owner_payload_ignores_unsupported_optional_values() {
    let options = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "deny-all",
            "nonInteractivePermissions": "ask",
            "authCredentials": ["not", "an", "object"],
            "authPolicy": "prompt",
            "suppressSdkConsoleErrors": "yes",
            "verbose": "no",
            "ttlMs": 1.5,
            "maxQueueDepth": "deep",
            "promptRetries": false
        }"#,
    )
    .expect("unsupported optional values should be ignored");

    assert_eq!(options.non_interactive_permissions, None);
    assert_eq!(options.auth_credentials, None);
    assert_eq!(options.auth_policy, None);
    assert_eq!(options.suppress_sdk_console_errors, None);
    assert_eq!(options.verbose, None);
    assert_eq!(options.ttl_ms, None);
    assert_eq!(options.max_queue_depth, None);
    assert_eq!(options.prompt_retries, None);
}

#[test]
fn parse_queue_owner_payload_rejects_invalid_json_and_non_object_payloads() {
    let json_error =
        parse_queue_owner_payload("{").expect_err("invalid json should fail").to_string();
    assert!(json_error.contains("EOF") || json_error.contains("expected"));

    let object_error = parse_queue_owner_payload("[]").expect_err("array payload should fail");
    assert_eq!(object_error.to_string(), "queue owner payload must be an object");
}

#[test]
fn parse_queue_owner_payload_rejects_missing_session_id() {
    let error = parse_queue_owner_payload(
        r#"{
            "permissionMode": "approve-all"
        }"#,
    )
    .expect_err("missing sessionId should fail");

    assert_eq!(error.to_string(), "queue owner payload missing sessionId");
}

#[test]
fn parse_queue_owner_payload_rejects_blank_session_id() {
    let error = parse_queue_owner_payload(
        r#"{
            "sessionId": "   ",
            "permissionMode": "approve-all"
        }"#,
    )
    .expect_err("blank sessionId should fail");

    assert_eq!(error.to_string(), "queue owner payload missing sessionId");
}

#[test]
fn parse_queue_owner_payload_rejects_non_string_session_id() {
    let error = parse_queue_owner_payload(
        r#"{
            "sessionId": 42,
            "permissionMode": "approve-all"
        }"#,
    )
    .expect_err("non-string sessionId should fail");

    assert_eq!(error.to_string(), "queue owner payload missing sessionId");
}

#[test]
fn parse_queue_owner_payload_rejects_invalid_permission_mode() {
    let error = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "invalid"
        }"#,
    )
    .expect_err("invalid permission mode should fail");

    assert_eq!(error.to_string(), "queue owner payload has invalid permissionMode");
}

#[test]
fn parse_queue_owner_payload_rejects_missing_or_non_string_permission_mode() {
    for payload in [
        r#"{
            "sessionId": "session-1"
        }"#,
        r#"{
            "sessionId": "session-1",
            "permissionMode": true
        }"#,
    ] {
        let error =
            parse_queue_owner_payload(payload).expect_err("invalid permission mode should fail");

        assert_eq!(error.to_string(), "queue owner payload has invalid permissionMode");
    }
}

#[test]
fn parse_queue_owner_payload_rejects_invalid_mcp_servers() {
    let error = parse_queue_owner_payload(
        r#"{
            "sessionId": "session-1",
            "permissionMode": "approve-all",
            "mcpServers": {}
        }"#,
    )
    .expect_err("invalid mcpServers should fail");

    assert_eq!(error.to_string(), "Invalid mcpServers in queue owner payload: expected array");
}

#[test]
fn queue_owner_runtime_options_from_env_returns_none_when_payload_env_is_missing() {
    let _lock = queue_owner_env_test_lock();
    let _env = EnvGuard::unset(QUEUE_OWNER_PAYLOAD_ENV);

    let options = queue_owner_runtime_options_from_env().expect("missing env should not fail");

    assert_eq!(options, None);
}

#[test]
fn queue_owner_runtime_options_from_env_rejects_blank_payload() {
    let _lock = queue_owner_env_test_lock();
    let _env = EnvGuard::set(QUEUE_OWNER_PAYLOAD_ENV, " \n\t ");

    let error = queue_owner_runtime_options_from_env().expect_err("blank payload should fail");

    assert!(matches!(error, QueueOwnerPayloadError::MissingPayload));
}

#[test]
fn queue_owner_runtime_options_from_env_parses_trimmed_payload() {
    let _lock = queue_owner_env_test_lock();
    let payload = format!(" \n{}\n ", minimal_payload("approve-reads"));
    let _env = EnvGuard::set(QUEUE_OWNER_PAYLOAD_ENV, &payload);

    let options = queue_owner_runtime_options_from_env()
        .expect("env payload should parse")
        .expect("payload should be present");

    assert_eq!(options.session_id, "session-1");
    assert_eq!(options.permission_mode, PermissionMode::ApproveReads);
}

#[test]
fn queue_owner_runtime_options_from_env_propagates_payload_parse_error() {
    let _lock = queue_owner_env_test_lock();
    let _env = EnvGuard::set(QUEUE_OWNER_PAYLOAD_ENV, "[]");

    let error = queue_owner_runtime_options_from_env().expect_err("invalid payload should fail");

    assert_eq!(error.to_string(), "queue owner payload must be an object");
}

#[test]
fn run_queue_owner_from_env_rejects_missing_or_blank_payload_before_runtime() {
    let _lock = queue_owner_env_test_lock();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build");
    let _missing = EnvGuard::unset(QUEUE_OWNER_PAYLOAD_ENV);

    let missing_error =
        runtime.block_on(run_queue_owner_from_env()).expect_err("missing payload should fail");
    assert!(matches!(
        missing_error,
        crate::QueueOwnerRunFromEnvError::Payload(QueueOwnerPayloadError::MissingPayload)
    ));

    drop(_missing);
    let _blank = EnvGuard::set(QUEUE_OWNER_PAYLOAD_ENV, " ");

    let blank_error =
        runtime.block_on(run_queue_owner_from_env()).expect_err("blank payload should fail");
    assert!(matches!(
        blank_error,
        crate::QueueOwnerRunFromEnvError::Payload(QueueOwnerPayloadError::MissingPayload)
    ));
}

#[test]
fn run_queue_owner_from_env_rejects_invalid_payload_before_runtime() {
    let _lock = queue_owner_env_test_lock();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build");
    let _env = EnvGuard::set(QUEUE_OWNER_PAYLOAD_ENV, "[]");

    let error =
        runtime.block_on(run_queue_owner_from_env()).expect_err("invalid payload should fail");

    assert!(matches!(
        error,
        crate::QueueOwnerRunFromEnvError::Payload(QueueOwnerPayloadError::InvalidPayload(_))
    ));
}
