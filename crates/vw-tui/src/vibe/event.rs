//! Product-neutral display events consumed by the copied Codex history cells.
//! 复用 Codex 历史单元格消费的产品中立展示事件。

/// A file attachment shown in a history entry.
/// 历史条目中展示的文件附件。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V3AttachmentKind {
    File,
    Image,
    PastedText,
    Resource,
}

#[derive(Clone, Eq, PartialEq)]
pub struct V3Attachment {
    pub kind: V3AttachmentKind,
    pub label: String,
    pub path: Option<String>,
    pub media_type: Option<String>,
    pub text: Option<String>,
    /// 由本地输入界面捕获、尚未提交的内存附件内容。
    pub bytes: Option<Vec<u8>>,
}

impl std::fmt::Debug for V3Attachment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("V3Attachment")
            .field("kind", &self.kind)
            .field("label", &self.label)
            .field("path", &self.path)
            .field("media_type", &self.media_type)
            .field("text", &self.text)
            .field("bytes_len", &self.bytes.as_ref().map(Vec::len))
            .finish()
    }
}

/// One product-neutral file change rendered by the TUI.
/// TUI 展示的单个产品中立文件变更。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V3FileChange {
    Add { content: String },
    Delete { content: String },
    Update { unified_diff: String },
}

/// A file path paired with its display-only change, in source arrival order.
/// 文件路径及其纯展示变更，顺序与来源事件一致。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3FileDiffEntry {
    pub path: String,
    pub change: V3FileChange,
}

/// Ordered file changes supplied by an outer bridge without filesystem access.
/// 外层桥接提供的有序文件变更，不授予文件系统访问能力。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V3FileDiff {
    pub files: Vec<V3FileDiffEntry>,
}

/// The display-only event vocabulary owned by VibeWindow.
///
/// This is deliberately not a wire-protocol enum.  An outer bridge may translate its
/// events into this shape, while the TUI remains a pure renderer with no request or approval
/// authority.
/// VibeWindow 自有的纯展示事件词汇；它不是线协议枚举，外层桥接可转换事件，而 TUI 保持无请求或审批权限的纯渲染器。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V3DisplayEvent {
    User { text: String, attachments: Vec<V3Attachment> },
    Assistant { markdown: String },
    ToolCall { call_id: String, tool: String, command: String },
    ToolResult {
        call_id: String,
        tool: String,
        command: String,
        output: String,
        exit_code: i32,
        duration_ms: u64,
    },
    FileDiff { diff: V3FileDiff },
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
/// `ChatWidget` 消费的产品中立流式生命周期输入；这是内部 UI 契约，不是 Gateway 或 app-server 线协议。
/// 外层 CLI 桥接将在迁移批次中把运行时通知翻译为这些确定性事件。
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
    FileDiff {
        turn_id: String,
        diff: V3FileDiff,
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
/// 一个计划或目标步骤的纯展示状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum V3StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
