//! 验证客户端辅助逻辑对会话选项、认证、错误和进程上下文的处理。

use std::collections::HashMap;
use std::error::Error as StdError;
use std::ffi::OsString;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use agent_client_protocol as acp;
use parking_lot::Mutex as ParkingMutex;
use serde_json::json;

use crate::AcpError;
use crate::errors::{AcpxErrorOptions, PermissionDeniedError, PermissionPromptUnavailableError};
use crate::types::{AcpSessionOptions, PermissionStats, PromptUsage};

use super::helpers::*;
use super::{ChildExitSummary, FinalizedChild};

static HELPERS_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn helpers_env_test_lock() -> MutexGuard<'static, ()> {
    HELPERS_ENV_TEST_LOCK.lock().expect("helpers env test lock should acquire")
}

struct EnvGuard {
    key: &'static str,
    original: Option<OsString>,
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

fn permission_stats() -> Arc<ParkingMutex<PermissionStats>> {
    Arc::new(ParkingMutex::new(PermissionStats::default()))
}

fn map_error(
    err: impl StdError + Send + Sync + 'static,
    permission_stats: &Arc<ParkingMutex<PermissionStats>>,
) -> acp::Error {
    map_client_error(Box::new(err), permission_stats)
}

#[test]
fn session_options_meta_filters_empty_values() {
    let meta = build_session_options_meta(Some(&AcpSessionOptions {
        model: Some(" claude-sonnet ".to_string()),
        allowed_tools: Some(vec!["Read".to_string(), " ".to_string(), "Write".to_string()]),
        max_turns: Some(3),
    }))
    .expect("meta");

    let options = &meta["claudeCode"]["options"];
    assert_eq!(options["model"], " claude-sonnet ");
    assert_eq!(options["allowedTools"], serde_json::json!(["Read", "Write"]));
    assert_eq!(options["maxTurns"], 3);
}

#[test]
fn session_options_meta_omits_empty_payload() {
    assert!(build_session_options_meta(None).is_none());
    assert!(
        build_session_options_meta(Some(&AcpSessionOptions {
            model: Some(" ".to_string()),
            allowed_tools: Some(vec![" ".to_string()]),
            max_turns: None,
        }))
        .is_none()
    );
}

#[test]
fn child_exit_summary_handles_missing_status() {
    let summary = child_exit_summary(None);

    assert_eq!(summary.exit_code, None);
    assert_eq!(summary.signal, None);
}

#[test]
fn auth_helpers_build_aliases_and_read_raw_env_first() {
    let _lock = helpers_env_test_lock();
    let _raw = EnvGuard::set("helpers-test-auth", "raw-secret");
    let _normalized = EnvGuard::set("HELPERS_TEST_AUTH", "normalized-secret");
    let _prefixed = EnvGuard::set("VWACP_AUTH_HELPERS_TEST_AUTH", "prefixed-secret");

    let mut credentials = HashMap::new();
    credentials.insert("helpers-test-auth".to_string(), "token".to_string());
    credentials.insert("bad=key".to_string(), "bad-token".to_string());
    credentials.insert("***".to_string(), "symbol-token".to_string());
    credentials.insert("blank".to_string(), " ".to_string());

    let env = build_agent_environment(&credentials);

    assert_eq!(env.get("helpers-test-auth").map(String::as_str), Some("token"));
    assert_eq!(env.get("HELPERS_TEST_AUTH").map(String::as_str), Some("token"));
    assert_eq!(env.get("VWACP_AUTH_HELPERS_TEST_AUTH").map(String::as_str), Some("token"));
    assert!(!env.contains_key("bad=key"));
    assert_eq!(env.get("BAD_KEY").map(String::as_str), Some("bad-token"));
    assert_eq!(env.get("***").map(String::as_str), Some("symbol-token"));
    assert!(!env.contains_key("blank"));
    assert_eq!(read_env_credential("helpers-test-auth").as_deref(), Some("raw-secret"));
    assert_eq!(to_env_token(" helpers.test/auth "), Some("HELPERS_TEST_AUTH".to_string()));
    assert_eq!(basename_token("/tmp/ACP-Agent"), "acp-agent");
}

#[test]
fn auth_helpers_read_aliases_and_reject_empty_tokens() {
    let _lock = helpers_env_test_lock();
    let _raw = EnvGuard::set("helpers-test-alias", " \t ");
    let _normalized = EnvGuard::unset("HELPERS_TEST_ALIAS");
    let _prefixed = EnvGuard::set("VWACP_AUTH_HELPERS_TEST_ALIAS", "prefixed-secret");

    assert_eq!(read_env_credential("helpers-test-alias").as_deref(), Some("prefixed-secret"));
    assert_eq!(to_env_token(" -- "), None);
    assert_eq!(basename_token(""), "");
}

#[test]
fn client_error_helpers_update_permission_stats_and_build_cancel_response() {
    let permission_stats = permission_stats();

    let mapped = map_error(
        PermissionDeniedError::new("write denied", AcpxErrorOptions::default()),
        &permission_stats,
    );
    let response = cancelled_permission_response();

    assert_eq!(permission_stats.lock().denied, 1);
    assert_eq!(permission_stats.lock().cancelled, 0);
    assert_eq!(mapped.code, acp::ErrorCode::InternalError);
    assert_eq!(mapped.data, Some(json!("write denied")));
    assert!(matches!(response.outcome, acp::RequestPermissionOutcome::Cancelled));
    assert!(response.meta.is_none());
}

#[test]
fn client_error_helpers_track_cancelled_and_ignore_unrelated_errors() {
    let permission_stats = permission_stats();

    let unavailable = map_error(PermissionPromptUnavailableError::new(), &permission_stats);
    let unrelated = map_error(std::io::Error::other("callback failed"), &permission_stats);

    assert_eq!(permission_stats.lock().denied, 0);
    assert_eq!(permission_stats.lock().cancelled, 1);
    assert_eq!(unavailable.code, acp::ErrorCode::InternalError);
    assert_eq!(
        unavailable.data,
        Some(json!("Permission prompt unavailable in non-interactive mode"))
    );
    assert_eq!(unrelated.data, Some(json!("callback failed")));
}

#[test]
fn process_context_helpers_enrich_supported_errors_and_wrap_acp_codes() {
    let finalized = FinalizedChild {
        summary: ChildExitSummary { exit_code: Some(9), signal: Some("SIGTERM".to_string()) },
        stderr_output: " first\nsecond ".to_string(),
    };

    let enriched = enrich_acp_error_with_process_context(
        AcpError::Initialize("failed".to_string()),
        &finalized,
    );
    let wrapped = wrap_session_control_error(
        "session/set_model",
        Some("for session abc".to_string()),
        std::io::Error::other(
            r#"JSON-RPC error -32601: method not found, "details":"missing method"}"#,
        ),
        AcpError::SetSessionModel,
    );

    assert_eq!(
        enriched.to_string(),
        "acp initialize failed: failed ACP agent process exited early (exit code 9, signal SIGTERM, stderr: first | second)"
    );
    assert_eq!(
        wrapped.to_string(),
        "Agent rejected session/set_model for session abc: missing method (ACP -32601, adapter reported \"method not found\"). The adapter may not implement session/set_model, or the requested value is not supported."
    );
}

#[test]
fn process_context_helpers_preserve_plain_failures_and_other_errors() {
    let finalized = FinalizedChild {
        summary: ChildExitSummary { exit_code: Some(2), signal: None },
        stderr_output: "stderr".to_string(),
    };

    let prompt_error =
        enrich_acp_error_with_process_context(AcpError::Prompt("failed".to_string()), &finalized);
    let wrapped_internal = wrap_session_control_error(
        "session/set_model",
        None,
        std::io::Error::other("JSON-RPC error -32603: internal error"),
        AcpError::SetSessionModel,
    );
    let wrapped_transport = wrap_session_control_error(
        "session/set_model",
        Some("for model x".to_string()),
        std::io::Error::other("transport closed"),
        AcpError::SetSessionModel,
    );

    assert_eq!(prompt_error.to_string(), "acp prompt failed: failed");
    assert_eq!(
        wrapped_internal.to_string(),
        "Failed session/set_model: internal error (ACP -32603)"
    );
    assert_eq!(
        wrapped_transport.to_string(),
        "Failed session/set_model for model x: transport closed"
    );
}

#[test]
fn process_signal_helpers_extend_targets_without_duplicates_or_zeroes() {
    let mut targets = ProcessCleanupTargets { process_groups: vec![10], processes: vec![20] };

    targets.extend(ProcessCleanupTargets {
        process_groups: vec![0, 10, 30],
        processes: vec![0, 20, 40],
    });

    assert_eq!(targets.process_groups, vec![10, 30]);
    assert_eq!(targets.processes, vec![20, 40]);
}

#[cfg(unix)]
#[test]
fn process_signal_helpers_parse_descendant_cleanup_targets() {
    let entries = parse_process_table_entries(
        r#"
        invalid row
        100 1 100
        101 100 101
        102 101 102
        103 100 100
        104 1 100
        105 104 105
        200 1 200
        "#,
    );

    let targets = process_cleanup_targets_from_entries(100, &entries);
    let mut process_groups = targets.process_groups;
    let mut processes = targets.processes;
    process_groups.sort_unstable();
    processes.sort_unstable();

    assert_eq!(process_groups, vec![100, 101, 102, 105]);
    assert_eq!(processes, vec![100, 101, 102, 103, 104, 105]);
}

#[test]
fn prompt_mapping_helpers_map_finish_reason_and_usage() {
    let mut usage = acp::Usage::new(18, 12, 6);
    usage.cached_read_tokens = Some(4);
    usage.thought_tokens = Some(3);

    assert_eq!(acp_finish_reason(acp::StopReason::Refusal), "refusal");
    assert_eq!(acp_finish_reason(acp::StopReason::EndTurn), "stop");
    assert_eq!(acp_finish_reason(acp::StopReason::MaxTokens), "length");
    assert_eq!(acp_finish_reason(acp::StopReason::MaxTurnRequests), "max_turn_requests");
    assert_eq!(acp_finish_reason(acp::StopReason::Cancelled), "cancelled");
    assert_eq!(
        map_usage(&usage),
        PromptUsage { input_tokens: 12, output_tokens: 6, cached_tokens: 4, reasoning_tokens: 3 }
    );
    assert_eq!(
        map_usage(&acp::Usage::new(1, 2, 3)),
        PromptUsage { input_tokens: 2, output_tokens: 3, cached_tokens: 0, reasoning_tokens: 0 }
    );
}

#[test]
fn timeout_message_helpers_include_diagnostics() {
    let gemini = build_gemini_acp_startup_timeout_message("gemini --experimental-acp");
    let claude = build_claude_acp_session_create_timeout_message();

    assert!(gemini.contains("Gemini CLI ACP startup timed out"));
    assert!(gemini.contains("Command: gemini --experimental-acp"));
    assert!(claude.contains("Claude ACP session creation timed out"));
    assert!(claude.contains("vwacp claude exec"));
}
