//! 跨模块共享的公共类型与输出协议定义。
//!
//! 本模块集中放置会被多个子系统共同依赖的基础类型，避免 CLI、队列、
//! 运行时和输出模块之间出现循环依赖。典型内容包括：
//! - 输出格式与输出上下文
//! - 会话恢复与权限模式等枚举
//! - 对外暴露的错误码和错误来源
//! - JSON-RPC 消息相关的公共载荷结构
//!
//! 这些类型是 crate 内部的通用语言层，修改时需要特别注意兼容性影响。

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::{
    AgentCapabilities, JsonRpcMessage, McpServer, SessionConfigOption, SessionModelState,
    SessionNotification, SetSessionConfigOptionResponse, StopReason,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_types::tools::ToolResultDto;

pub use agent_client_protocol::{
    McpServer as AcpMcpServer, SessionNotification as AcpSessionNotification,
};

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

pub type ExitCode = i32;

pub const EXIT_CODE_SUCCESS: ExitCode = 0;
pub const EXIT_CODE_ERROR: ExitCode = 1;
pub const EXIT_CODE_USAGE: ExitCode = 2;
pub const EXIT_CODE_TIMEOUT: ExitCode = 3;
pub const EXIT_CODE_NO_SESSION: ExitCode = 4;
pub const EXIT_CODE_PERMISSION_DENIED: ExitCode = 5;
pub const EXIT_CODE_INTERRUPTED: ExitCode = 130;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionMode {
    #[default]
    ApproveAll,
    ApproveReads,
    DenyAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthPolicy {
    Skip,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NonInteractivePermissionPolicy {
    Deny,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionResumePolicy {
    AllowNew,
    SameSessionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStream {
    Prompt,
    Control,
}

pub type AcpJsonRpcMessage = JsonRpcMessage<Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AcpMessageDirection {
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputErrorCode {
    #[serde(rename = "NO_SESSION")]
    NoSession,
    #[serde(rename = "TIMEOUT")]
    Timeout,
    #[serde(rename = "PERMISSION_DENIED")]
    PermissionDenied,
    #[serde(rename = "PERMISSION_PROMPT_UNAVAILABLE")]
    PermissionPromptUnavailable,
    #[serde(rename = "RUNTIME")]
    Runtime,
    #[serde(rename = "USAGE")]
    Usage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputErrorOrigin {
    Cli,
    Runtime,
    Queue,
    Acp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueErrorDetailCode {
    #[serde(rename = "QUEUE_OWNER_CLOSED")]
    QueueOwnerClosed,
    #[serde(rename = "QUEUE_OWNER_SHUTTING_DOWN")]
    QueueOwnerShuttingDown,
    #[serde(rename = "QUEUE_OWNER_OVERLOADED")]
    QueueOwnerOverloaded,
    #[serde(rename = "QUEUE_OWNER_GENERATION_MISMATCH")]
    QueueOwnerGenerationMismatch,
    #[serde(rename = "QUEUE_REQUEST_INVALID")]
    QueueRequestInvalid,
    #[serde(rename = "QUEUE_REQUEST_PAYLOAD_INVALID_JSON")]
    QueueRequestPayloadInvalidJson,
    #[serde(rename = "QUEUE_ACK_MISSING")]
    QueueAckMissing,
    #[serde(rename = "QUEUE_DISCONNECTED_BEFORE_ACK")]
    QueueDisconnectedBeforeAck,
    #[serde(rename = "QUEUE_DISCONNECTED_BEFORE_COMPLETION")]
    QueueDisconnectedBeforeCompletion,
    #[serde(rename = "QUEUE_PROTOCOL_INVALID_JSON")]
    QueueProtocolInvalidJson,
    #[serde(rename = "QUEUE_PROTOCOL_MALFORMED_MESSAGE")]
    QueueProtocolMalformedMessage,
    #[serde(rename = "QUEUE_PROTOCOL_UNEXPECTED_RESPONSE")]
    QueueProtocolUnexpectedResponse,
    #[serde(rename = "QUEUE_NOT_ACCEPTING_REQUESTS")]
    QueueNotAcceptingRequests,
    #[serde(rename = "QUEUE_CONTROL_REQUEST_FAILED")]
    QueueControlRequestFailed,
    #[serde(rename = "QUEUE_RUNTIME_PROMPT_FAILED")]
    QueueRuntimePromptFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputErrorAcpPayload {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionStats {
    pub requested: i64,
    pub approved: i64,
    pub denied: i64,
    pub cancelled: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientOperationMethod {
    #[serde(rename = "fs/read_text_file")]
    FsReadTextFile,
    #[serde(rename = "fs/write_text_file")]
    FsWriteTextFile,
    #[serde(rename = "terminal/create")]
    TerminalCreate,
    #[serde(rename = "terminal/output")]
    TerminalOutput,
    #[serde(rename = "terminal/wait_for_exit")]
    TerminalWaitForExit,
    #[serde(rename = "terminal/kill")]
    TerminalKill,
    #[serde(rename = "terminal/release")]
    TerminalRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientOperationStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientOperation {
    pub method: ClientOperationMethod,
    pub status: ClientOperationStatus,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEventLog {
    pub active_path: String,
    pub segment_count: i64,
    pub max_segment_bytes: i64,
    pub max_segments: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_write_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_write_error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfMetricSummary {
    pub count: i64,
    #[serde(rename = "totalMs")]
    pub total_ms: f64,
    #[serde(rename = "maxMs")]
    pub max_ms: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfMetricsSnapshot {
    pub counters: HashMap<String, i64>,
    pub timings: HashMap<String, PerfMetricSummary>,
    pub gauges: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputFormatterContext {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicy {
    pub format: OutputFormat,
    #[serde(rename = "jsonStrict")]
    pub json_strict: bool,
    #[serde(rename = "suppressReads")]
    pub suppress_reads: bool,
    #[serde(rename = "suppressNonJsonStderr")]
    pub suppress_non_json_stderr: bool,
    #[serde(rename = "queueErrorAlreadyEmitted")]
    pub queue_error_already_emitted: bool,
    #[serde(rename = "suppressSdkConsoleErrors")]
    pub suppress_sdk_console_errors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputErrorEmissionPolicy {
    #[serde(rename = "queueErrorAlreadyEmitted")]
    pub queue_error_already_emitted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputErrorParams {
    pub code: OutputErrorCode,
    #[serde(rename = "detailCode", default, skip_serializing_if = "Option::is_none")]
    pub detail_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<OutputErrorOrigin>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp: Option<OutputErrorAcpPayload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

pub trait OutputFormatter {
    fn set_context(&mut self, context: OutputFormatterContext);
    fn on_acp_message(&mut self, message: AcpJsonRpcMessage);
    fn on_error(&mut self, params: OutputErrorParams);
    fn flush(&mut self);
}

pub type AcpMessageCallback = Arc<dyn Fn(AcpMessageDirection, AcpJsonRpcMessage) + Send + Sync>;
pub type SessionUpdateCallback = Arc<dyn Fn(SessionNotification) + Send + Sync>;
pub type ClientOperationCallback = Arc<dyn Fn(ClientOperation) + Send + Sync>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionOptions {
    pub model: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<i64>,
}

#[derive(Clone, Default)]
pub struct AcpClientOptions {
    pub agent_command: String,
    pub cwd: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub suppress_sdk_console_errors: bool,
    pub verbose: bool,
    pub session_options: Option<AcpSessionOptions>,
    pub on_acp_message: Option<AcpMessageCallback>,
    pub on_acp_output_message: Option<AcpMessageCallback>,
    pub on_session_update: Option<SessionUpdateCallback>,
    pub on_client_operation: Option<ClientOperationCallback>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpAgentConfig {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStrategy {
    New,
    Load(String),
    Resume(String),
    ResumeOrLoad(String),
    ResumeLoadOrNew(String),
    LoadOrNew(String),
}

#[derive(Debug, Clone)]
pub struct PromptRequest {
    pub cwd: PathBuf,
    pub prompt: String,
    pub session_strategy: SessionStrategy,
}

impl PromptRequest {
    pub fn new(cwd: PathBuf, prompt: impl Into<String>) -> Self {
        Self { cwd, prompt: prompt.into(), session_strategy: SessionStrategy::New }
    }

    pub fn with_session_strategy(mut self, session_strategy: SessionStrategy) -> Self {
        self.session_strategy = session_strategy;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub session_id: String,
    pub models: Option<SessionModelState>,
    pub config_options: Option<Vec<SessionConfigOption>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptEvent {
    TextDelta(String),
    SessionChanged { expected: String, actual: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub reasoning_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptResult {
    pub session_id: String,
    pub deltas: Vec<String>,
    pub finish_reason: Option<String>,
    pub usage: Option<PromptUsage>,
}

pub const SESSION_RECORD_SCHEMA: &str = "vwacp.session.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMessageImageSize {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMessageImage {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<SessionMessageImageSize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMention {
    pub uri: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionUserContent {
    Text(String),
    Mention(SessionMention),
    Image(SessionMessageImage),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionToolUse {
    pub id: String,
    pub name: String,
    pub raw_input: String,
    pub input: Value,
    pub is_input_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionToolResultContent {
    Text(String),
    Image(SessionMessageImage),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionToolResult {
    pub tool_use_id: String,
    pub tool_name: String,
    pub is_error: bool,
    pub content: SessionToolResultContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ToolResultDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionThinking {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionAgentContent {
    Text(String),
    Thinking(SessionThinking),
    RedactedThinking(String),
    ToolUse(SessionToolUse),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUserMessage {
    pub id: String,
    pub content: Vec<SessionUserContent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionAgentMessage {
    pub content: Vec<SessionAgentContent>,
    pub tool_results: HashMap<String, SessionToolResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_details: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionMessage {
    User(SessionUserMessage),
    Agent(SessionAgentMessage),
    Resume,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTokenUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionConversation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub updated_at: String,
    pub cumulative_token_usage: SessionTokenUsage,
    pub request_token_usage: HashMap<String, SessionTokenUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionStateOptions {
    pub model: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionAcpxState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_mode_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_mode_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_models: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_commands: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_options: Option<Vec<SessionConfigOption>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_config_options: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_options: Option<SessionStateOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub schema: String,
    #[serde(rename = "vwacpRecordId")]
    pub vwacp_record_id: String,
    #[serde(rename = "acpSessionId")]
    pub acp_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "agentSessionId")]
    pub agent_session_id: Option<String>,
    #[serde(rename = "agentCommand")]
    pub agent_command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "agentConfig")]
    pub agent_config: Option<AcpAgentConfig>,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastUsedAt")]
    pub last_used_at: String,
    #[serde(rename = "lastSeq")]
    pub last_seq: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastRequestId")]
    pub last_request_id: Option<String>,
    #[serde(rename = "eventLog")]
    pub event_log: SessionEventLog,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "closedAt")]
    pub closed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "agentStartedAt")]
    pub agent_started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastPromptAt")]
    pub last_prompt_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastAgentExitCode")]
    pub last_agent_exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastAgentExitSignal")]
    pub last_agent_exit_signal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastAgentExitAt")]
    pub last_agent_exit_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "lastAgentDisconnectReason")]
    pub last_agent_disconnect_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "protocolVersion")]
    pub protocol_version: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "agentCapabilities")]
    pub agent_capabilities: Option<AgentCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub updated_at: String,
    pub cumulative_token_usage: SessionTokenUsage,
    pub request_token_usage: HashMap<String, SessionTokenUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vwacp: Option<SessionAcpxState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPromptResult {
    pub stop_reason: StopReason,
    pub permission_stats: PermissionStats,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSendResult {
    pub stop_reason: StopReason,
    pub permission_stats: PermissionStats,
    pub session_id: String,
    pub record: SessionRecord,
    pub resumed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSetModeResult {
    pub record: SessionRecord,
    pub resumed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSetConfigOptionResult {
    pub record: SessionRecord,
    pub response: SetSessionConfigOptionResponse,
    pub resumed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSetModelResult {
    pub record: SessionRecord,
    pub resumed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEnsureResult {
    pub record: SessionRecord,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEnqueueResult {
    pub queued: bool,
    pub session_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionSendOutcome {
    SessionSendResult(Box<SessionSendResult>),
    SessionEnqueueResult(SessionEnqueueResult),
}
