//! Product-neutral display events consumed by the copied Codex history cells.

/// A file attachment shown in a history entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3Attachment {
    pub label: String,
    pub path: Option<String>,
}

/// The display-only event vocabulary owned by VibeWindow.
///
/// This is deliberately not a wire-protocol enum.  An outer bridge may translate its
/// events into this shape, while the TUI remains a pure renderer with no request or approval
/// authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V3DisplayEvent {
    User { text: String, attachments: Vec<V3Attachment> },
    Assistant { markdown: String },
    ToolCall { call_id: String, tool: String, command: String },
    ToolResult { call_id: String, output: String, exit_code: i32, duration_ms: u64 },
    Thinking { text: String },
    Step { title: String, status: V3StepStatus },
    System { text: String },
    Error { message: String },
    Attachment { attachment: V3Attachment },
    Progress { label: String, detail: Option<String> },
    CompactBoundary { summary: String },
    Unknown { type_name: String, detail: String },
}

/// Product-neutral streaming lifecycle input consumed by `ChatWidget`.
///
/// This is an internal UI contract, not a Gateway or app-server wire type. The
/// outer CLI bridge translates runtime notifications into these deterministic
/// events in the next migration batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V3Event {
    TurnStarted {
        turn_id: String,
    },
    ThinkingDelta {
        turn_id: String,
        delta: String,
    },
    TextDelta {
        turn_id: String,
        delta: String,
    },
    ToolStarted {
        turn_id: String,
        call_id: String,
        tool: String,
        command: String,
    },
    ToolOutputDelta {
        turn_id: String,
        call_id: String,
        delta: String,
    },
    ToolCompleted {
        turn_id: String,
        call_id: String,
        output: String,
        exit_code: i32,
        duration_ms: u64,
    },
    ToolFailed {
        turn_id: String,
        call_id: String,
        message: String,
        duration_ms: u64,
    },
    ToolCancelled {
        turn_id: String,
        call_id: String,
        reason: String,
        duration_ms: u64,
    },
    TurnCompleted {
        turn_id: String,
    },
    TurnFailed {
        turn_id: String,
        message: String,
    },
    TurnCancelled {
        turn_id: String,
        reason: String,
    },
    Unknown {
        type_name: String,
        detail: String,
    },
}

/// Display status for one plan/goal step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V3StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
