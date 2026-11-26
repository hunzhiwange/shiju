//! Project-scoped CLI workbench contracts.
use serde::{Deserialize, Serialize};
pub use vw_api_acp_model::acp_model_source::AcpModelSource;

/// Process-local connection to the session's workbench-only gateway listener.
pub const CHILD_CONTEXT_ENV: &str = "SHIJU_WORKBENCH_CONTEXT";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliWorkbenchChildContext {
    pub port: u16,
    pub project_id: String,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CliWorkbenchTarget {
    Acp { agent: String },
    NativeCli { app_id: String },
    Shell,
    Ssh { connection_id: String },
}

/// Desktop surface that owns an SSH tab. Older sessions belong to the app terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SshSessionSurface {
    Terminal,
    AppTerminal,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshConnectionFields {
    pub name: String,
    pub host: String,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshConnection {
    pub id: String,
    pub from_config: bool,
    pub fields: SshConnectionFields,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SshConnections {
    pub saved: Vec<SshConnection>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SshConnectionChange {
    Save { id: Option<String>, fields: SshConnectionFields },
    Import { aliases: Vec<String> },
    Delete { id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWorkbenchDeliveryCapability {
    Structured,
    Draft,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchCapabilities {
    pub available: bool,
    pub terminal: bool,
    pub delivery: CliWorkbenchDeliveryCapability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWorkbenchProcessState {
    /// The session accepts a turn. A newly created ACP session launches on first delivery.
    Ready,
    Running,
    WaitingForApproval,
    Stopping,
    Exited,
    Disconnected,
}

/// Task activity reported by a native agent, distinct from PTY process liveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWorkbenchAgentActivity {
    Idle,
    Working,
    NeedsInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchSession {
    pub id: String,
    pub project_id: String,
    pub directory: String,
    pub target: CliWorkbenchTarget,
    pub state: CliWorkbenchProcessState,
    pub capabilities: CliWorkbenchCapabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_activity: Option<CliWorkbenchAgentActivity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_surface: Option<SshSessionSurface>,
    /// Recent structured deliveries, in submission order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_request_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchProject {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub sessions: Vec<CliWorkbenchSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchProjects {
    pub projects: Vec<CliWorkbenchProject>,
    pub preferred_project: Option<String>,
    pub terminal_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartCliWorkbenchSession {
    pub target: CliWorkbenchTarget,
    /// Defaults to the registered project root. May also name a registered worktree.
    pub directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_surface: Option<SshSessionSurface>,
    /// Required for ACP targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp: Option<CliWorkbenchAcpOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliWorkbenchAcpOptions {
    pub model_source: AcpModelSource,
    /// Optional session-model override, as `provider/model`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendCliWorkbenchMessage {
    pub request_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchMessageId {
    pub request_id: String,
    pub sender_user_id: String,
    pub target_session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWorkbenchMessageState {
    Accepted,
    PendingSubmission,
    Submitted,
    WaitingForApproval,
    Completed,
    Failed,
}

impl CliWorkbenchMessageState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWorkbenchErrorCode {
    InvalidRequest,
    Unauthorized,
    NotFound,
    RequestConflict,
    Closed,
    AwaitingApproval,
    Busy,
    Unavailable,
    Disconnected,
    Cancelled,
    TimedOut,
    TurnFailed,
    GatewayFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchError {
    pub code: CliWorkbenchErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchMessage {
    pub id: CliWorkbenchMessageId,
    /// Gateway-created identifier for this submission, independent of the request ID.
    pub turn_id: String,
    pub state: CliWorkbenchMessageState,
    /// A human submits this draft in the terminal. There is no tracked reply.
    #[serde(default)]
    pub manual_submission: bool,
    /// Increases whenever the message projection changes.
    pub revision: u64,
    pub approval_request_id: Option<String>,
    /// Retains the IDs of approvals encountered in this turn after resolution.
    pub approval_history: Vec<String>,
    pub assistant_message_id: Option<String>,
    pub output: String,
    pub output_truncated: bool,
    pub error: Option<CliWorkbenchErrorCode>,
}

impl CliWorkbenchMessage {
    pub fn is_settled(&self) -> bool {
        self.state.is_terminal()
            || (self.manual_submission && self.state == CliWorkbenchMessageState::Submitted)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchTerminalCapability {
    pub available: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native_apps: Vec<CliWorkbenchNativeApp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchNativeApp {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliWorkbenchTerminalSize {
    pub rows: u16,
    pub cols: u16,
}

/// Output offsets are absolute for the life of the gateway-owned PTY.
#[derive(Clone, Serialize, Deserialize)]
pub struct CliWorkbenchTerminalOutput {
    pub bytes: Vec<u8>,
    pub start: u64,
    pub next: u64,
    pub truncated: bool,
    pub finished: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliWorkbenchTerminalInput {
    pub bytes: Vec<u8>,
    pub size: Option<CliWorkbenchTerminalSize>,
}

pub const CLI_WORKBENCH_MAX_TERMINAL_INPUT: usize = 64 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct CliWorkbenchDraft {
    pub id: CliWorkbenchMessageId,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliWorkbenchDraftAction {
    MarkSubmitted,
    Cancel,
}
