//! 运行时、队列与权限相关的扩展错误类型集合。

use std::error::Error as StdError;
use std::fmt;
use std::ops::Deref;

use crate::types::{OutputErrorAcpPayload, OutputErrorCode, OutputErrorOrigin, OutputErrorParams};

pub type ErrorSource = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Default)]
pub struct AcpxErrorOptions {
    pub source: Option<ErrorSource>,
    pub output_code: Option<OutputErrorCode>,
    pub detail_code: Option<String>,
    pub origin: Option<OutputErrorOrigin>,
    pub retryable: Option<bool>,
    pub acp: Option<OutputErrorAcpPayload>,
    pub output_already_emitted: bool,
}

impl AcpxErrorOptions {
    pub fn with_defaults(
        mut self,
        output_code: OutputErrorCode,
        detail_code: impl Into<String>,
        origin: OutputErrorOrigin,
    ) -> Self {
        if self.output_code.is_none() {
            self.output_code = Some(output_code);
        }
        if self.detail_code.is_none() {
            self.detail_code = Some(detail_code.into());
        }
        if self.origin.is_none() {
            self.origin = Some(origin);
        }
        self
    }
}

#[derive(Debug)]
pub struct AcpxOperationalError {
    message: String,
    source: Option<ErrorSource>,
    output_code: Option<OutputErrorCode>,
    detail_code: Option<String>,
    origin: Option<OutputErrorOrigin>,
    retryable: Option<bool>,
    acp: Option<OutputErrorAcpPayload>,
    output_already_emitted: bool,
}

impl AcpxOperationalError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            message: message.into(),
            source: options.source,
            output_code: options.output_code,
            detail_code: options.detail_code,
            origin: options.origin,
            retryable: options.retryable,
            acp: options.acp,
            output_already_emitted: options.output_already_emitted,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn output_code(&self) -> Option<OutputErrorCode> {
        self.output_code
    }

    pub fn detail_code(&self) -> Option<&str> {
        self.detail_code.as_deref()
    }

    pub fn origin(&self) -> Option<OutputErrorOrigin> {
        self.origin
    }

    pub fn retryable(&self) -> Option<bool> {
        self.retryable
    }

    pub fn acp(&self) -> Option<&OutputErrorAcpPayload> {
        self.acp.as_ref()
    }

    pub fn output_already_emitted(&self) -> bool {
        self.output_already_emitted
    }

    pub fn to_output_error_params(&self) -> Option<OutputErrorParams> {
        Some(OutputErrorParams {
            code: self.output_code?,
            detail_code: self.detail_code.clone(),
            origin: self.origin,
            message: self.message.clone(),
            retryable: self.retryable,
            acp: self.acp.clone(),
            timestamp: None,
        })
    }
}

impl fmt::Display for AcpxOperationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for AcpxOperationalError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|error| error as &(dyn StdError + 'static))
    }
}

macro_rules! impl_operational_error_wrapper {
    ($name:ident) => {
        impl Deref for $name {
            type Target = AcpxOperationalError;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.inner, f)
            }
        }

        impl StdError for $name {
            fn source(&self) -> Option<&(dyn StdError + 'static)> {
                self.inner.source()
            }
        }
    };
}

#[derive(Debug)]
pub struct SessionNotFoundError {
    pub session_id: String,
    inner: AcpxOperationalError,
}

impl SessionNotFoundError {
    pub fn new(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let message = format!("Session not found: {session_id}");
        Self { session_id, inner: AcpxOperationalError::new(message, AcpxErrorOptions::default()) }
    }
}

impl_operational_error_wrapper!(SessionNotFoundError);

#[derive(Debug)]
pub struct SessionResolutionError {
    inner: AcpxOperationalError,
}

impl SessionResolutionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { inner: AcpxOperationalError::new(message, AcpxErrorOptions::default()) }
    }
}

impl_operational_error_wrapper!(SessionResolutionError);

#[derive(Debug)]
pub struct AgentSpawnError {
    pub agent_command: String,
    inner: AcpxOperationalError,
}

impl AgentSpawnError {
    pub fn new(
        agent_command: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        let agent_command = agent_command.into();
        let message = format!("Failed to spawn agent command: {agent_command}");
        Self {
            agent_command,
            inner: AcpxOperationalError::new(
                message,
                AcpxErrorOptions { source: Some(Box::new(source)), ..AcpxErrorOptions::default() },
            ),
        }
    }
}

impl_operational_error_wrapper!(AgentSpawnError);

#[derive(Debug)]
pub struct AgentDisconnectedError {
    pub reason: String,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    inner: AcpxOperationalError,
}

impl AgentDisconnectedError {
    pub fn new(
        reason: impl Into<String>,
        exit_code: Option<i32>,
        signal: Option<String>,
        options: AcpxErrorOptions,
    ) -> Self {
        let reason = reason.into();
        let message = format!(
            "ACP agent disconnected during request ({reason}, exit={}, signal={})",
            exit_code.map_or_else(|| "null".to_string(), |value| value.to_string()),
            signal.clone().unwrap_or_else(|| "null".to_string()),
        );
        Self {
            reason,
            exit_code,
            signal,
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Runtime,
                    "AGENT_DISCONNECTED",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(AgentDisconnectedError);

#[derive(Debug)]
pub struct SessionResumeRequiredError {
    inner: AcpxOperationalError,
}

impl SessionResumeRequiredError {
    pub fn new(message: impl Into<String>, mut options: AcpxErrorOptions) -> Self {
        if options.retryable.is_none() {
            options.retryable = Some(true);
        }
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Runtime,
                    "SESSION_RESUME_REQUIRED",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(SessionResumeRequiredError);

#[derive(Debug)]
pub struct GeminiAcpStartupTimeoutError {
    inner: AcpxOperationalError,
}

impl GeminiAcpStartupTimeoutError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Timeout,
                    "GEMINI_ACP_STARTUP_TIMEOUT",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(GeminiAcpStartupTimeoutError);

#[derive(Debug)]
pub struct SessionModeReplayError {
    inner: AcpxOperationalError,
}

impl SessionModeReplayError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Runtime,
                    "SESSION_MODE_REPLAY_FAILED",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(SessionModeReplayError);

#[derive(Debug)]
pub struct SessionModelReplayError {
    inner: AcpxOperationalError,
}

impl SessionModelReplayError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Runtime,
                    "SESSION_MODEL_REPLAY_FAILED",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(SessionModelReplayError);

#[derive(Debug)]
pub struct ClaudeAcpSessionCreateTimeoutError {
    inner: AcpxOperationalError,
}

impl ClaudeAcpSessionCreateTimeoutError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Timeout,
                    "CLAUDE_ACP_SESSION_CREATE_TIMEOUT",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(ClaudeAcpSessionCreateTimeoutError);

#[derive(Debug)]
pub struct CopilotAcpUnsupportedError {
    inner: AcpxOperationalError,
}

impl CopilotAcpUnsupportedError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Runtime,
                    "COPILOT_ACP_UNSUPPORTED",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(CopilotAcpUnsupportedError);

#[derive(Debug)]
pub struct AuthPolicyError {
    inner: AcpxOperationalError,
}

impl AuthPolicyError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self {
            inner: AcpxOperationalError::new(
                message,
                options.with_defaults(
                    OutputErrorCode::Runtime,
                    "AUTH_REQUIRED",
                    OutputErrorOrigin::Acp,
                ),
            ),
        }
    }
}

impl_operational_error_wrapper!(AuthPolicyError);

#[derive(Debug)]
pub struct QueueConnectionError {
    inner: Box<AcpxOperationalError>,
}

impl QueueConnectionError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self { inner: Box::new(AcpxOperationalError::new(message, options)) }
    }
}

impl Deref for QueueConnectionError {
    type Target = AcpxOperationalError;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl fmt::Display for QueueConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.inner.as_ref(), f)
    }
}

impl StdError for QueueConnectionError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source()
    }
}

#[derive(Debug)]
pub struct QueueProtocolError {
    inner: AcpxOperationalError,
}

impl QueueProtocolError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self { inner: AcpxOperationalError::new(message, options) }
    }
}

impl_operational_error_wrapper!(QueueProtocolError);

#[derive(Debug)]
pub struct PermissionDeniedError {
    inner: AcpxOperationalError,
}

impl PermissionDeniedError {
    pub fn new(message: impl Into<String>, options: AcpxErrorOptions) -> Self {
        Self { inner: AcpxOperationalError::new(message, options) }
    }
}

impl_operational_error_wrapper!(PermissionDeniedError);

#[derive(Debug)]
pub struct PermissionPromptUnavailableError {
    inner: AcpxOperationalError,
}

impl PermissionPromptUnavailableError {
    pub fn new() -> Self {
        Self {
            inner: AcpxOperationalError::new(
                "Permission prompt unavailable in non-interactive mode",
                AcpxErrorOptions::default(),
            ),
        }
    }
}

impl Default for PermissionPromptUnavailableError {
    fn default() -> Self {
        Self::new()
    }
}

impl_operational_error_wrapper!(PermissionPromptUnavailableError);
