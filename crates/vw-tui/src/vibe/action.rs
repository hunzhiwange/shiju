//! Actions emitted by VibeWindow-owned TUI surfaces.

use super::event::V3Attachment;

/// Read-only row data supplied by VibeWindow-owned surfaces.
///
/// Selection widgets deliberately receive this value instead of resolving commands,
/// sessions, files, themes, or settings themselves.  The UI only filters and renders it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct V3SelectionItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub search_value: Option<String>,
    pub disabled: bool,
    pub disabled_reason: Option<String>,
    pub is_current: bool,
    pub is_default: bool,
}

impl V3SelectionItem {
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), label: label.into(), ..Self::default() }
    }
}

/// A file result supplied by VibeWindow's file-search boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct V3FileSearchMatch {
    pub id: String,
    pub path: String,
    pub match_indices: Vec<usize>,
}

/// The security-sensitive surface being shown by an approval overlay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V3ApprovalKind {
    Command,
    FileChange,
    Network,
    Permissions,
    Elicitation,
}

/// A decision selected in an approval overlay.
///
/// This is intentionally a UI value.  The CLI bridge decides how, or whether, to turn it into a
/// runtime permission response.  In particular, rendering this value never grants access.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V3ApprovalDecision {
    Allow,
    AllowForSession,
    Deny,
    Cancel,
    Custom(String),
}

/// A read-only approval request supplied by the VibeWindow bridge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3ApprovalRequest {
    pub id: String,
    pub kind: V3ApprovalKind,
    pub title: String,
    pub detail: String,
    pub choices: Vec<V3ApprovalChoice>,
}

impl V3ApprovalRequest {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        kind: V3ApprovalKind,
        title: impl Into<String>,
        detail: impl Into<String>,
        choices: Vec<V3ApprovalChoice>,
    ) -> Self {
        Self { id: id.into(), kind, title: title.into(), detail: detail.into(), choices }
    }
}

/// One visible approval choice, including explicit reject and cancel paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3ApprovalChoice {
    pub id: String,
    pub label: String,
    pub decision: V3ApprovalDecision,
}

impl V3ApprovalChoice {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        decision: V3ApprovalDecision,
    ) -> Self {
        Self { id: id.into(), label: label.into(), decision }
    }
}

/// A single question supplied to the user-input overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3UserInputQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Vec<V3UserInputOption>,
    pub allow_other: bool,
    pub secret: bool,
}

impl V3UserInputQuestion {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        header: impl Into<String>,
        question: impl Into<String>,
        options: Vec<V3UserInputOption>,
    ) -> Self {
        Self {
            id: id.into(),
            header: header.into(),
            question: question.into(),
            options,
            allow_other: false,
            secret: false,
        }
    }
}

/// One read-only option for a user-input question.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3UserInputOption {
    pub label: String,
    pub description: String,
}

impl V3UserInputOption {
    #[must_use]
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self { label: label.into(), description: description.into() }
    }
}

/// A read-only user-input request supplied by the VibeWindow bridge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3UserInputRequest {
    pub id: String,
    pub questions: Vec<V3UserInputQuestion>,
}

impl V3UserInputRequest {
    #[must_use]
    pub fn new(id: impl Into<String>, questions: Vec<V3UserInputQuestion>) -> Self {
        Self { id: id.into(), questions }
    }
}

/// A submitted answer for one question.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3UserInputAnswer {
    pub question_id: String,
    pub values: Vec<String>,
}

/// Non-security selection surfaces that share the copied list renderer.
pub const V3_NON_SECURITY_SELECTION_SURFACES: &[&str] = &[
    "search",
    "todo",
    "task",
    "command_palette",
    "report",
    "mcp",
    "memory",
    "settings",
    "permissions",
    "integration",
    "background_tasks",
    "agents",
    "workflows",
    "schedules",
];

/// Read-only data supplied by the CLI bridge to the composer footer.
///
/// The composer never resolves these values itself.  Keeping them as plain display data makes
/// the UI independent from Gateway clients, runtime handles, and Codex configuration state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct V3FooterData {
    pub model: Option<String>,
    pub permission: Option<String>,
    pub directory: Option<String>,
    pub context_window_percent: Option<u8>,
    pub context_window_used_tokens: Option<u64>,
    pub is_task_running: bool,
}

/// Business-facing results emitted by the copied composer/footer.
///
/// `None` means the key only changed local editor state.  No variant carries a runtime or
/// request object: the outer CLI bridge owns interpretation and side effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V3Action {
    None,
    Select {
        surface: String,
        item_id: String,
    },
    Submit {
        text: String,
        attachments: Vec<V3Attachment>,
    },
    Queue {
        text: String,
        attachments: Vec<V3Attachment>,
    },
    Interrupt,
    Cancel,
    HistoryPrevious,
    HistoryNext,
    Attach(V3Attachment),
    RemoveAttachment {
        index: usize,
    },
    ToggleShortcuts,
    OpenExternalEditor,
    ShowTranscript,
    InsertNewline,
    /// The bridge must validate and route this decision; the TUI does not reply to a gateway.
    ResolveApproval {
        request_id: String,
        decision: V3ApprovalDecision,
    },
    /// Answers leave the UI as data; no protocol response is constructed here.
    AnswerUserInput {
        request_id: String,
        answers: Vec<V3UserInputAnswer>,
    },
}
