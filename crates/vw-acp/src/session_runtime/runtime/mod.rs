//! 会话创建、发送与取消等运行时主流程实现。
//!
//! 本模块提供会话层最外侧的运行时 API，包括：
//! - 创建或确保会话存在
//! - 向指定会话发送 prompt
//! - 取消正在执行的任务
//! - 在需要时绕过队列直接与运行中的会话交互
//!
//! 它是 CLI 和队列服务端最终调用的主流程入口之一。

use std::collections::HashMap;

use agent_client_protocol::McpServer;

use crate::AcpClient;
use crate::prompt_content::PromptInput;
use crate::types::{
    AcpAgentConfig, AcpMessageCallback, AcpSessionOptions, AuthPolicy, ClientOperationCallback,
    NonInteractivePermissionPolicy, OutputErrorEmissionPolicy, OutputFormatter, PermissionMode,
    SessionRecord, SessionResumePolicy, SessionUpdateCallback,
};

mod error;
mod prompt;
mod queue_owner;
mod queue_owner_controls;
mod session_records;
mod state;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use error::SessionRuntimeError;
pub use prompt::run_once;
#[allow(unused_imports)]
pub use queue_owner::{
    cancel_session_prompt, normalize_queue_owner_ttl_ms, run_session_queue_owner, send_session,
    send_session_direct,
};
#[allow(unused_imports)]
pub use session_records::{create_session, create_session_with_client, ensure_session};

/// 队列所有者默认租约存活时间，单位为毫秒。
pub const DEFAULT_QUEUE_OWNER_TTL_MS: u64 = 300_000;
pub(super) const QUEUE_OWNER_STARTUP_MAX_ATTEMPTS: usize = 120;
pub(super) const QUEUE_OWNER_HEARTBEAT_INTERVAL_MS: u64 = 5_000;

/// 单次运行会话主流程所需的参数集合。
///
/// 该结构用于把一次 prompt 发送过程中所需的 agent 命令、会话记录、
/// 输出回调、权限配置和队列控制参数集中传递给运行时入口。
pub struct RunOnceOptions<'a> {
    pub agent_command: String,
    pub agent_config: Option<AcpAgentConfig>,
    pub cwd: String,
    pub prompt: PromptInput,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub output_formatter: &'a mut dyn OutputFormatter,
    pub on_acp_message: Option<AcpMessageCallback>,
    pub on_session_update: Option<SessionUpdateCallback>,
    pub on_client_operation: Option<ClientOperationCallback>,
    pub suppress_sdk_console_errors: bool,
    pub verbose: bool,
    pub session_options: Option<AcpSessionOptions>,
    pub prompt_retries: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Clone)]
pub struct SessionCreateOptions {
    pub agent_command: String,
    pub agent_config: Option<AcpAgentConfig>,
    pub cwd: String,
    pub name: Option<String>,
    pub resume_session_id: Option<String>,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub verbose: bool,
    pub session_options: Option<AcpSessionOptions>,
    pub timeout_ms: Option<u64>,
}

pub struct SessionSendOptions<'a> {
    pub session_id: String,
    pub prompt: PromptInput,
    pub resume_policy: Option<SessionResumePolicy>,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub output_formatter: &'a mut dyn OutputFormatter,
    pub on_acp_message: Option<AcpMessageCallback>,
    pub on_session_update: Option<SessionUpdateCallback>,
    pub on_client_operation: Option<ClientOperationCallback>,
    pub error_emission_policy: Option<OutputErrorEmissionPolicy>,
    pub suppress_sdk_console_errors: bool,
    pub verbose: bool,
    pub wait_for_completion: bool,
    pub ttl_ms: Option<u64>,
    pub max_queue_depth: Option<usize>,
    pub prompt_retries: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Clone)]
pub struct SessionEnsureOptions {
    pub agent_command: String,
    pub agent_config: Option<AcpAgentConfig>,
    pub cwd: String,
    pub name: Option<String>,
    pub resume_session_id: Option<String>,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub verbose: bool,
    pub walk_boundary: Option<String>,
    pub session_options: Option<AcpSessionOptions>,
    pub timeout_ms: Option<u64>,
}

#[derive(Clone)]
pub struct SessionCancelOptions {
    pub session_id: String,
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCancelResult {
    pub session_id: String,
    pub cancelled: bool,
}

#[derive(Clone)]
pub struct SessionCreateWithClientResult {
    pub record: SessionRecord,
    pub client: AcpClient,
}
