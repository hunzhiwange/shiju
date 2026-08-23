//! 验证操作错误的默认元数据、错误源和输出参数契约。

use std::error::Error as StdError;
use std::fmt::Display;
use std::io;
use std::ops::Deref;

use super::errors::{
    AcpxErrorOptions, AcpxOperationalError, AgentDisconnectedError, AgentSpawnError,
    AuthPolicyError, ClaudeAcpSessionCreateTimeoutError, CopilotAcpUnsupportedError,
    GeminiAcpStartupTimeoutError, PermissionDeniedError, PermissionPromptUnavailableError,
    QueueConnectionError, QueueProtocolError, SessionModeReplayError, SessionModelReplayError,
    SessionNotFoundError, SessionResolutionError, SessionResumeRequiredError,
};
use super::types::{OutputErrorAcpPayload, OutputErrorCode, OutputErrorOrigin};

struct ExpectedOutput<'a> {
    message: &'a str,
    code: OutputErrorCode,
    detail_code: Option<&'a str>,
    origin: Option<OutputErrorOrigin>,
    retryable: Option<bool>,
}

fn assert_output_params<T>(error: &T, expected: ExpectedOutput<'_>)
where
    T: Deref<Target = AcpxOperationalError> + Display + StdError,
{
    assert_eq!(error.message(), expected.message);
    assert_eq!(error.to_string(), expected.message);
    assert_eq!(error.output_code(), Some(expected.code));
    assert_eq!(error.detail_code(), expected.detail_code);
    assert_eq!(error.origin(), expected.origin);
    assert_eq!(error.retryable(), expected.retryable);

    let params = error.to_output_error_params().expect("output params");
    assert_eq!(params.code, expected.code);
    assert_eq!(params.detail_code.as_deref(), expected.detail_code);
    assert_eq!(params.origin, expected.origin);
    assert_eq!(params.message, expected.message);
    assert_eq!(params.retryable, expected.retryable);
    assert_eq!(params.timestamp, None);
}

fn assert_no_output_params<T>(error: &T, expected_message: &str)
where
    T: Deref<Target = AcpxOperationalError> + Display + StdError,
{
    assert_eq!(error.message(), expected_message);
    assert_eq!(error.to_string(), expected_message);
    assert!(error.to_output_error_params().is_none());
}

#[test]
fn options_with_defaults_preserves_explicit_values() {
    let options = AcpxErrorOptions {
        output_code: Some(OutputErrorCode::Timeout),
        detail_code: Some("EXPLICIT".to_string()),
        origin: Some(OutputErrorOrigin::Queue),
        ..AcpxErrorOptions::default()
    }
    .with_defaults(OutputErrorCode::Runtime, "DEFAULT", OutputErrorOrigin::Acp);

    assert_eq!(options.output_code, Some(OutputErrorCode::Timeout));
    assert_eq!(options.detail_code.as_deref(), Some("EXPLICIT"));
    assert_eq!(options.origin, Some(OutputErrorOrigin::Queue));
}

#[test]
fn options_with_defaults_fills_missing_values() {
    let options = AcpxErrorOptions::default().with_defaults(
        OutputErrorCode::Runtime,
        "DEFAULT_DETAIL",
        OutputErrorOrigin::Acp,
    );

    assert_eq!(options.output_code, Some(OutputErrorCode::Runtime));
    assert_eq!(options.detail_code.as_deref(), Some("DEFAULT_DETAIL"));
    assert_eq!(options.origin, Some(OutputErrorOrigin::Acp));
}

#[test]
fn operational_error_exposes_output_fields_and_source() {
    let acp =
        OutputErrorAcpPayload { code: -32603, message: "adapter failed".to_string(), data: None };
    let error = AcpxOperationalError::new(
        "visible message",
        AcpxErrorOptions {
            source: Some(Box::new(io::Error::other("inner"))),
            output_code: Some(OutputErrorCode::Runtime),
            detail_code: Some("DETAIL".to_string()),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: Some(true),
            acp: Some(acp.clone()),
            output_already_emitted: true,
        },
    );

    assert_eq!(error.message(), "visible message");
    assert_eq!(error.to_string(), "visible message");
    assert_eq!(error.output_code(), Some(OutputErrorCode::Runtime));
    assert_eq!(error.detail_code(), Some("DETAIL"));
    assert_eq!(error.origin(), Some(OutputErrorOrigin::Acp));
    assert_eq!(error.retryable(), Some(true));
    assert_eq!(error.acp(), Some(&acp));
    assert!(error.output_already_emitted());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn output_error_params_require_output_code() {
    let error = AcpxOperationalError::new("message", AcpxErrorOptions::default());

    assert!(error.to_output_error_params().is_none());
}

#[test]
fn session_lookup_errors_expose_message_without_output_params() {
    let not_found = SessionNotFoundError::new("session-1");
    assert_eq!(not_found.session_id, "session-1");
    assert_no_output_params(&not_found, "Session not found: session-1");
    assert!(StdError::source(&not_found).is_none());

    let resolution = SessionResolutionError::new("ambiguous session");
    assert_no_output_params(&resolution, "ambiguous session");
    assert!(StdError::source(&resolution).is_none());
}

#[test]
fn agent_spawn_error_exposes_command_and_source() {
    let error = AgentSpawnError::new("agent --stdio", io::Error::other("spawn failed"));

    assert_eq!(error.agent_command, "agent --stdio");
    assert_no_output_params(&error, "Failed to spawn agent command: agent --stdio");
    assert_eq!(StdError::source(&error).map(ToString::to_string).as_deref(), Some("spawn failed"));
}

#[test]
fn agent_disconnected_error_applies_defaults_for_status_message() {
    let error = AgentDisconnectedError::new(
        "request failed",
        Some(7),
        Some("TERM".to_string()),
        AcpxErrorOptions::default(),
    );

    assert_eq!(error.reason, "request failed");
    assert_eq!(error.exit_code, Some(7));
    assert_eq!(error.signal.as_deref(), Some("TERM"));
    assert_output_params(
        &error,
        ExpectedOutput {
            message: "ACP agent disconnected during request (request failed, exit=7, signal=TERM)",
            code: OutputErrorCode::Runtime,
            detail_code: Some("AGENT_DISCONNECTED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );
}

#[test]
fn agent_disconnected_error_preserves_explicit_output_options() {
    let error = AgentDisconnectedError::new(
        "closed",
        None,
        None,
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::PermissionDenied),
            detail_code: Some("CUSTOM_DISCONNECT".to_string()),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(false),
            ..AcpxErrorOptions::default()
        },
    );

    assert_eq!(error.exit_code, None);
    assert_eq!(error.signal, None);
    assert_output_params(
        &error,
        ExpectedOutput {
            message: "ACP agent disconnected during request (closed, exit=null, signal=null)",
            code: OutputErrorCode::PermissionDenied,
            detail_code: Some("CUSTOM_DISCONNECT"),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(false),
        },
    );
}

#[test]
fn resume_required_error_defaults_to_retryable() {
    let error = SessionResumeRequiredError::new("resume required", AcpxErrorOptions::default());

    assert_output_params(
        &error,
        ExpectedOutput {
            message: "resume required",
            code: OutputErrorCode::Runtime,
            detail_code: Some("SESSION_RESUME_REQUIRED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: Some(true),
        },
    );
}

#[test]
fn resume_required_error_preserves_explicit_retryable() {
    let error = SessionResumeRequiredError::new(
        "do not retry",
        AcpxErrorOptions { retryable: Some(false), ..AcpxErrorOptions::default() },
    );

    assert_output_params(
        &error,
        ExpectedOutput {
            message: "do not retry",
            code: OutputErrorCode::Runtime,
            detail_code: Some("SESSION_RESUME_REQUIRED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: Some(false),
        },
    );
}

#[test]
fn acp_timeout_errors_apply_expected_output_metadata() {
    let gemini = GeminiAcpStartupTimeoutError::new("gemini timeout", AcpxErrorOptions::default());
    assert_output_params(
        &gemini,
        ExpectedOutput {
            message: "gemini timeout",
            code: OutputErrorCode::Timeout,
            detail_code: Some("GEMINI_ACP_STARTUP_TIMEOUT"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );

    let claude =
        ClaudeAcpSessionCreateTimeoutError::new("claude timeout", AcpxErrorOptions::default());
    assert_output_params(
        &claude,
        ExpectedOutput {
            message: "claude timeout",
            code: OutputErrorCode::Timeout,
            detail_code: Some("CLAUDE_ACP_SESSION_CREATE_TIMEOUT"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );
}

#[test]
fn acp_replay_and_policy_errors_apply_expected_output_metadata() {
    let mode = SessionModeReplayError::new("mode replay failed", AcpxErrorOptions::default());
    assert_output_params(
        &mode,
        ExpectedOutput {
            message: "mode replay failed",
            code: OutputErrorCode::Runtime,
            detail_code: Some("SESSION_MODE_REPLAY_FAILED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );

    let model = SessionModelReplayError::new("model replay failed", AcpxErrorOptions::default());
    assert_output_params(
        &model,
        ExpectedOutput {
            message: "model replay failed",
            code: OutputErrorCode::Runtime,
            detail_code: Some("SESSION_MODEL_REPLAY_FAILED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );

    let copilot = CopilotAcpUnsupportedError::new("unsupported", AcpxErrorOptions::default());
    assert_output_params(
        &copilot,
        ExpectedOutput {
            message: "unsupported",
            code: OutputErrorCode::Runtime,
            detail_code: Some("COPILOT_ACP_UNSUPPORTED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );

    let auth = AuthPolicyError::new("auth required", AcpxErrorOptions::default());
    assert_output_params(
        &auth,
        ExpectedOutput {
            message: "auth required",
            code: OutputErrorCode::Runtime,
            detail_code: Some("AUTH_REQUIRED"),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: None,
        },
    );
}

#[test]
fn queue_connection_error_exposes_boxed_operational_error() {
    let error = QueueConnectionError::new(
        "queue failed",
        AcpxErrorOptions {
            source: Some(Box::new(io::Error::other("socket closed"))),
            output_code: Some(OutputErrorCode::Runtime),
            detail_code: Some("QUEUE_FAILED".to_string()),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(true),
            ..AcpxErrorOptions::default()
        },
    );

    assert_output_params(
        &error,
        ExpectedOutput {
            message: "queue failed",
            code: OutputErrorCode::Runtime,
            detail_code: Some("QUEUE_FAILED"),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(true),
        },
    );
    assert_eq!(StdError::source(&error).map(ToString::to_string).as_deref(), Some("socket closed"));
}

#[test]
fn queue_protocol_and_permission_denied_errors_preserve_options() {
    let protocol = QueueProtocolError::new(
        "bad queue message",
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::Runtime),
            detail_code: Some("QUEUE_PROTOCOL_BAD_MESSAGE".to_string()),
            origin: Some(OutputErrorOrigin::Queue),
            ..AcpxErrorOptions::default()
        },
    );
    assert_output_params(
        &protocol,
        ExpectedOutput {
            message: "bad queue message",
            code: OutputErrorCode::Runtime,
            detail_code: Some("QUEUE_PROTOCOL_BAD_MESSAGE"),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: None,
        },
    );

    let denied = PermissionDeniedError::new(
        "permission denied",
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::PermissionDenied),
            detail_code: Some("DENIED".to_string()),
            origin: Some(OutputErrorOrigin::Runtime),
            retryable: Some(false),
            ..AcpxErrorOptions::default()
        },
    );
    assert_output_params(
        &denied,
        ExpectedOutput {
            message: "permission denied",
            code: OutputErrorCode::PermissionDenied,
            detail_code: Some("DENIED"),
            origin: Some(OutputErrorOrigin::Runtime),
            retryable: Some(false),
        },
    );
}

#[test]
fn permission_prompt_unavailable_has_stable_default_message() {
    let created = PermissionPromptUnavailableError::new();
    assert_no_output_params(&created, "Permission prompt unavailable in non-interactive mode");

    let defaulted = PermissionPromptUnavailableError::default();
    assert_no_output_params(&defaulted, "Permission prompt unavailable in non-interactive mode");
}
