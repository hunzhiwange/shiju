//! Project-scoped CLI workbench contracts.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CliWorkbenchTarget {
    Acp { agent: String },
    NativeCli { app_id: String },
    Shell,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliWorkbenchSession {
    pub id: String,
    pub project_id: String,
    pub directory: String,
    pub target: CliWorkbenchTarget,
    pub state: CliWorkbenchProcessState,
    pub capabilities: CliWorkbenchCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartCliWorkbenchSession {
    pub target: CliWorkbenchTarget,
    /// Defaults to the registered project root. May also name a registered worktree.
    pub directory: Option<String>,
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
