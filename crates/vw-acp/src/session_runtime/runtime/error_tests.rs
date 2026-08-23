//! 验证运行时错误的中断识别、输出归一化和回退元数据应用。

use super::*;
use crate::OutputErrorCode;
use crate::errors::{AcpxErrorOptions, AcpxOperationalError};
use crate::types::OutputErrorAcpPayload;

#[derive(Debug, thiserror::Error)]
#[error("outer: {0}")]
struct OuterError(#[source] InterruptedError);

#[derive(Debug, thiserror::Error)]
#[error("plain failure")]
struct PlainError;

#[derive(Debug, thiserror::Error)]
#[error("outer queue wrapper: {0}")]
struct OuterQueueError(#[source] QueueConnectionError);

#[derive(Debug)]
struct TestOperationalError(AcpxOperationalError);

impl std::ops::Deref for TestOperationalError {
    type Target = AcpxOperationalError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for TestOperationalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

#[test]
fn from_source_marks_interrupted_errors_in_source_chain() {
    let error = SessionRuntimeError::from_source(OuterError(InterruptedError));

    assert!(error.is_interrupted());
    assert_eq!(error.to_string(), "outer: Interrupted");
    assert!(error.source().is_some());
}

#[test]
fn output_already_emitted_is_configurable() {
    let error = SessionRuntimeError::from_source(PlainError).with_output_already_emitted(true);

    assert!(error.output_already_emitted());
}

#[test]
fn normalize_runtime_error_maps_timeout_to_retryable_timeout_output() {
    let output = normalize_runtime_error(
        &TimeoutError { timeout_ms: 25 },
        Some("detail"),
        Some(OutputErrorOrigin::Runtime),
    );

    assert_eq!(output.code, OutputErrorCode::Timeout);
    assert_eq!(output.detail_code.as_deref(), Some("detail"));
    assert_eq!(output.retryable, Some(true));
    assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
}

#[test]
fn output_params_uses_runtime_fallback_for_plain_errors() {
    let output = SessionRuntimeError::from_source(PlainError).output_params();

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
    assert_eq!(output.message, "plain failure");
}

#[test]
fn output_params_falls_back_when_normalized_output_is_absent() {
    let error = SessionRuntimeError {
        message: "stored failure".to_string(),
        source: None,
        output: None,
        output_already_emitted: false,
        interrupted: false,
    };

    let output = error.output_params();

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
    assert_eq!(output.message, "stored failure");
}

#[test]
fn source_returns_none_for_runtime_error_without_source() {
    let error = SessionRuntimeError {
        message: "stored failure".to_string(),
        source: None,
        output: None,
        output_already_emitted: false,
        interrupted: false,
    };

    assert!(error.source().is_none());
    assert!(!error.is_interrupted());
    assert!(!error.output_already_emitted());
}

#[test]
fn normalize_runtime_error_uses_queue_connection_output_metadata() {
    let acp = OutputErrorAcpPayload { code: -32603, message: "queue acp".to_string(), data: None };
    let error = QueueConnectionError::new(
        "queue unavailable",
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::Timeout),
            detail_code: Some("QUEUE_TIMEOUT".to_string()),
            origin: Some(OutputErrorOrigin::Queue),
            retryable: Some(true),
            acp: Some(acp.clone()),
            ..AcpxErrorOptions::default()
        },
    );

    let output =
        normalize_runtime_error(&error, Some("fallback"), Some(OutputErrorOrigin::Runtime));

    assert_eq!(output.code, OutputErrorCode::Timeout);
    assert_eq!(output.detail_code.as_deref(), Some("QUEUE_TIMEOUT"));
    assert_eq!(output.origin, Some(OutputErrorOrigin::Queue));
    assert_eq!(output.message, "queue unavailable");
    assert_eq!(output.retryable, Some(true));
    assert_eq!(output.acp, Some(acp));
}

#[test]
fn normalize_runtime_error_applies_fallback_metadata_to_queue_connection_error() {
    let error = QueueConnectionError::new("queue failed", AcpxErrorOptions::default());

    let output =
        normalize_runtime_error(&error, Some("FALLBACK_DETAIL"), Some(OutputErrorOrigin::Runtime));

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.detail_code.as_deref(), Some("FALLBACK_DETAIL"));
    assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
    assert_eq!(output.message, "queue failed");
    assert_eq!(output.retryable, None);
    assert_eq!(output.acp, None);
}

#[test]
fn normalize_runtime_error_finds_queue_connection_error_in_source_chain() {
    let inner = QueueConnectionError::new(
        "inner queue",
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::Runtime),
            detail_code: Some("INNER_QUEUE".to_string()),
            origin: Some(OutputErrorOrigin::Queue),
            ..AcpxErrorOptions::default()
        },
    );
    let error = OuterQueueError(inner);

    let output = normalize_runtime_error(&error, None, Some(OutputErrorOrigin::Runtime));

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.detail_code.as_deref(), Some("INNER_QUEUE"));
    assert_eq!(output.origin, Some(OutputErrorOrigin::Queue));
    assert_eq!(output.message, "inner queue");
}

#[test]
fn normalize_runtime_error_keeps_queue_connection_without_origin_when_no_fallback_exists() {
    let error = QueueConnectionError::new("queue failed", AcpxErrorOptions::default());

    let output = normalize_runtime_error(&error, None, None);

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.detail_code, None);
    assert_eq!(output.origin, None);
    assert_eq!(output.message, "queue failed");
}

#[test]
fn normalize_runtime_error_maps_operational_session_errors() {
    let resume = SessionResumeRequiredError::new("resume first", AcpxErrorOptions::default());
    let mode = SessionModeReplayError::new("mode replay", AcpxErrorOptions::default());
    let model = SessionModelReplayError::new("model replay", AcpxErrorOptions::default());

    let resume_output = normalize_runtime_error(&resume, None, None);
    assert_eq!(resume_output.code, OutputErrorCode::Runtime);
    assert_eq!(resume_output.detail_code.as_deref(), Some("SESSION_RESUME_REQUIRED"));
    assert_eq!(resume_output.origin, Some(OutputErrorOrigin::Acp));
    assert_eq!(resume_output.message, "resume first");
    assert_eq!(resume_output.retryable, Some(true));

    let mode_output = normalize_runtime_error(&mode, None, None);
    assert_eq!(mode_output.detail_code.as_deref(), Some("SESSION_MODE_REPLAY_FAILED"));
    assert_eq!(mode_output.origin, Some(OutputErrorOrigin::Acp));
    assert_eq!(mode_output.message, "mode replay");

    let model_output = normalize_runtime_error(&model, None, None);
    assert_eq!(model_output.detail_code.as_deref(), Some("SESSION_MODEL_REPLAY_FAILED"));
    assert_eq!(model_output.origin, Some(OutputErrorOrigin::Acp));
    assert_eq!(model_output.message, "model replay");
}

#[test]
fn operational_output_applies_fallback_metadata_when_fields_are_missing() {
    let error = TestOperationalError(AcpxOperationalError::new(
        "partial operational",
        AcpxErrorOptions {
            output_code: Some(OutputErrorCode::Runtime),
            ..AcpxErrorOptions::default()
        },
    ));

    let output =
        operational_output(&error, Some("FALLBACK_DETAIL"), Some(OutputErrorOrigin::Runtime));

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.detail_code.as_deref(), Some("FALLBACK_DETAIL"));
    assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
    assert_eq!(output.message, "partial operational");
}

#[test]
fn operational_output_uses_runtime_fallback_when_output_params_are_absent() {
    let error = TestOperationalError(AcpxOperationalError::new(
        "opaque operational",
        AcpxErrorOptions::default(),
    ));

    let output = operational_output(&error, Some("OPAQUE"), Some(OutputErrorOrigin::Runtime));

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.detail_code.as_deref(), Some("OPAQUE"));
    assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
    assert_eq!(output.message, "opaque operational");
}

#[test]
fn normalize_runtime_error_maps_acp_session_errors_to_no_session() {
    for error in
        [AcpError::LoadSession("missing".to_string()), AcpError::ResumeSession("gone".to_string())]
    {
        let output =
            normalize_runtime_error(&error, Some("ACP_DETAIL"), Some(OutputErrorOrigin::Runtime));

        assert_eq!(output.code, OutputErrorCode::NoSession);
        assert_eq!(output.detail_code.as_deref(), Some("ACP_DETAIL"));
        assert_eq!(output.origin, Some(OutputErrorOrigin::Runtime));
        assert_eq!(output.retryable, None);
    }
}

#[test]
fn normalize_runtime_error_maps_general_acp_errors_to_acp_origin_runtime_output() {
    let output = normalize_runtime_error(&AcpError::Prompt("failed".to_string()), None, None);

    assert_eq!(output.code, OutputErrorCode::Runtime);
    assert_eq!(output.detail_code, None);
    assert_eq!(output.origin, Some(OutputErrorOrigin::Acp));
    assert_eq!(output.message, "acp prompt failed: failed");
}
