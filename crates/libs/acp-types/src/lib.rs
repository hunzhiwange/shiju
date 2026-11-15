//! ACP 协议、共享类型与错误定义。

#![allow(clippy::result_large_err)]

pub mod types;
pub mod prompt_content;
pub mod acp_jsonrpc;
pub mod agent_session_id;
pub mod runtime_session_id;
pub mod session_mounts;
pub mod error;
pub mod errors;
pub mod acp_error_shapes;
pub mod jsonrpc_error;
pub mod error_normalization;
pub mod session_event_log;

pub use types::*;
pub use prompt_content::{
    PromptInput, PromptInputValidationError, is_prompt_input, merge_prompt_source_with_text,
    parse_prompt_source, prompt_to_display_text, text_prompt,
};
pub use acp_jsonrpc::{
    extract_session_update_notification, is_acp_json_rpc_message, is_json_rpc_notification,
    is_session_update_notification, parse_json_rpc_error_message, parse_prompt_stop_reason,
};
pub use agent_session_id::{
    AGENT_SESSION_ID_META_KEYS, extract_agent_session_id, normalize_agent_session_id,
};
pub use runtime_session_id::{
    RUNTIME_SESSION_ID_META_KEYS, extract_runtime_session_id, normalize_runtime_session_id,
};
pub use session_mounts::*;
pub use error::AcpError;
pub use errors::{
    AcpxErrorOptions, AcpxOperationalError, AgentDisconnectedError, AgentSpawnError,
    AuthPolicyError, ClaudeAcpSessionCreateTimeoutError, CopilotAcpUnsupportedError, ErrorSource,
    GeminiAcpStartupTimeoutError, PermissionDeniedError, PermissionPromptUnavailableError,
    QueueConnectionError, QueueProtocolError, SessionModeReplayError, SessionModelReplayError,
    SessionNotFoundError, SessionResolutionError, SessionResumeRequiredError,
};
pub use acp_error_shapes::{
    extract_acp_error, format_unknown_error_message, is_acp_resource_not_found_error,
};
pub use jsonrpc_error::{
    BuildJsonRpcErrorParams, JsonRpcErrorObject, JsonRpcErrorResponse,
    build_json_rpc_error_response, output_error_jsonrpc_code,
};
pub use error_normalization::{
    NormalizeOutputErrorOptions, exit_code_for_output_error_code, format_error_message,
    is_acp_query_closed_before_response_error, is_retryable_prompt_error, normalize_output_error,
};
pub use session_event_log::{
    DEFAULT_EVENT_MAX_SEGMENTS, DEFAULT_EVENT_SEGMENT_MAX_BYTES, default_session_base_dir,
    default_session_event_active_path, default_session_event_lock_path, default_session_event_log,
    default_session_event_segment_path, safe_session_id, session_base_dir,
    session_event_active_path, session_event_lock_path, session_event_log,
    session_event_segment_path,
};

#[cfg(test)]
#[path = "error_normalization_tests.rs"]
mod error_normalization_tests;

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;

#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;

#[cfg(test)]
#[path = "jsonrpc_error_tests.rs"]
mod jsonrpc_error_tests;
