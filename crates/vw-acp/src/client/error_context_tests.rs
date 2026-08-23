//! 验证子进程上下文对协议错误的补充、截断与兼容分类。
//!
//! 人工构造的退出摘要用于精确控制退出码、信号和标准错误片段。

use crate::AcpError;

use super::helpers::*;
use super::{ChildExitSummary, FinalizedChild};

fn finalized_child(
    exit_code: Option<i32>,
    signal: Option<&str>,
    stderr_output: &str,
) -> FinalizedChild {
    FinalizedChild {
        summary: ChildExitSummary { exit_code, signal: signal.map(str::to_string) },
        stderr_output: stderr_output.to_string(),
    }
}

fn wrap_set_model(raw: &str, context: Option<&str>) -> AcpError {
    wrap_session_control_error(
        "session/set_model",
        context.map(str::to_string),
        std::io::Error::other(raw),
        AcpError::SetSessionModel,
    )
}

// 下列行为用例覆盖上下文补充、预览截断和不支持方法的兼容识别。
#[test]
fn enrich_initialize_error_includes_exit_code_signal_and_trimmed_stderr() {
    let err = AcpError::Initialize("handshake failed".to_string());
    let finalized = finalized_child(Some(7), Some("SIGTERM"), "  first line\nsecond line  ");

    let enriched = enrich_acp_error_with_process_context(err, &finalized);

    assert_eq!(
        enriched.to_string(),
        "acp initialize failed: handshake failed ACP agent process exited early (exit code 7, signal SIGTERM, stderr: first line | second line)"
    );
}

#[test]
fn enrich_new_session_error_truncates_long_stderr_preview() {
    let err = AcpError::NewSession("connection closed".to_string());
    let stderr_output = format!("{}tail", "x".repeat(401));
    let finalized = finalized_child(None, None, &stderr_output);

    let enriched = enrich_acp_error_with_process_context(err, &finalized);
    let message = enriched.to_string();

    assert!(message.contains(&"x".repeat(400)));
    assert!(!message.contains("tail"));
}

#[test]
fn enrich_other_error_keeps_original_variant_even_with_process_context() {
    let err = AcpError::Prompt("prompt failed".to_string());
    let finalized = finalized_child(Some(1), Some("SIGKILL"), "stderr");

    let enriched = enrich_acp_error_with_process_context(err, &finalized);

    assert_eq!(enriched.to_string(), "acp prompt failed: prompt failed");
}

#[test]
fn enrich_error_without_context_keeps_original_message() {
    let err = AcpError::Initialize("connection closed".to_string());
    let finalized = FinalizedChild::default();

    let enriched = enrich_acp_error_with_process_context(err, &finalized);

    assert_eq!(enriched.to_string(), "acp initialize failed: connection closed");
}

#[test]
fn wrap_session_control_method_not_found_reports_unsupported_method() {
    let err = wrap_set_model(
        r#"JSON-RPC error -32601: method not found, "details":"session/set_model missing"}"#,
        Some("for session abc"),
    );

    assert_eq!(
        err.to_string(),
        "Agent rejected session/set_model for session abc: session/set_model missing (ACP -32601, adapter reported \"method not found\"). The adapter may not implement session/set_model, or the requested value is not supported."
    );
}

#[test]
fn wrap_session_control_invalid_params_reports_unsupported_value() {
    let err = wrap_set_model(
        r#"JSON-RPC error -32602: invalid params, "details": "model is unsupported"}"#,
        None,
    );

    assert_eq!(
        err.to_string(),
        "Agent rejected session/set_model: model is unsupported (ACP -32602, adapter reported \"invalid params\"). The adapter may not implement session/set_model, or the requested value is not supported."
    );
}

#[test]
fn wrap_session_control_single_quoted_details_reports_unsupported_value() {
    let err = wrap_set_model(
        "JSON-RPC error -32602: invalid params, details: 'model alias is unavailable'",
        Some("for model alias"),
    );

    assert_eq!(
        err.to_string(),
        "Agent rejected session/set_model for model alias: model alias is unavailable (ACP -32602, adapter reported \"invalid params\"). The adapter may not implement session/set_model, or the requested value is not supported."
    );
}

#[test]
fn wrap_session_control_internal_error_with_invalid_params_details_reports_unsupported_value() {
    let err = wrap_set_model(
        "JSON-RPC error -32603: internal error, details: invalid params: mode unavailable",
        Some("for mode plan"),
    );

    assert_eq!(
        err.to_string(),
        "Agent rejected session/set_model for mode plan: invalid params: mode unavailable (ACP -32603, adapter reported \"invalid params\"). The adapter may not implement session/set_model, or the requested value is not supported."
    );
}

#[test]
fn wrap_session_control_internal_error_without_invalid_params_is_failure() {
    let err = wrap_set_model(
        "JSON-RPC error -32603: internal error, details: adapter crashed",
        Some("for session abc"),
    );

    assert_eq!(
        err.to_string(),
        "Failed session/set_model for session abc: adapter crashed (ACP -32603, adapter reported \"internal error\")"
    );
}

#[test]
fn wrap_session_control_known_code_without_known_message_uses_raw_summary() {
    let err = wrap_set_model("adapter returned -32601 without a label", None);

    assert_eq!(
        err.to_string(),
        "Agent rejected session/set_model: adapter returned -32601 without a label (ACP -32601). The adapter may not implement session/set_model, or the requested value is not supported."
    );
}

#[test]
fn wrap_session_control_acp_error_without_details_uses_code_summary() {
    let err = wrap_set_model("JSON-RPC error -32603: internal error", None);

    assert_eq!(err.to_string(), "Failed session/set_model: internal error (ACP -32603)");
}

#[test]
fn wrap_session_control_malformed_details_falls_back_to_code_summary() {
    let err = wrap_set_model("JSON-RPC error -32603: internal error, details: ", None);

    assert_eq!(err.to_string(), "Failed session/set_model: internal error (ACP -32603)");
}

#[test]
fn wrap_session_control_non_acp_error_preserves_raw_message() {
    let err = wrap_set_model("transport closed", Some("for session abc"));

    assert_eq!(err.to_string(), "Failed session/set_model for session abc: transport closed");
}
