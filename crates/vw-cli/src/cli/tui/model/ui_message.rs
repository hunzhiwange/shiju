//! 新 TUI 的内部消息模型。
//!
//! 这里不直接复用 `ChatSession` 的扁平消息外形，而是为 CLI 新 TUI 定义更细粒度的
//! `UiMessage` 主枚举，承接后续 event -> message 标准化与 grouped/collapsed 渲染。

use vw_shared::session::ui_types as session_ui;

use super::attachment::UiAttachment;

/// 新 TUI 内部使用的消息标识符。
///
/// 当前先统一用字符串承载，调用方可自行选择 `gateway:*` 或 `local:*` 命名空间，
/// 避免在 Phase 2 过早引入额外 id 分配器。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UiMessageId(String);

impl UiMessageId {
    /// 使用既有原始值创建内部消息 ID。
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 为本地尚未持久化的消息创建 `local:*` 形式的 ID。
    pub(crate) fn local(seed: impl Into<String>) -> Self {
        Self(format!("local:{}", seed.into()))
    }

    /// 为 gateway 已分配消息创建 `gateway:*` 形式的 ID。
    pub(crate) fn gateway(seed: impl Into<String>) -> Self {
        Self(format!("gateway:{}", seed.into()))
    }

    /// 返回底层字符串值。
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// 所有 UI 消息共享的元信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiMessageBase {
    pub(crate) id: UiMessageId,
    pub(crate) parent_id: Option<UiMessageId>,
    pub(crate) session_id: Option<String>,
    pub(crate) created_ms: Option<u64>,
    pub(crate) origin: Option<UiMessageOrigin>,
}

impl UiMessageBase {
    /// 创建一条最小消息元信息。
    pub(crate) fn new(id: UiMessageId) -> Self {
        Self { id, parent_id: None, session_id: None, created_ms: None, origin: None }
    }

    /// 绑定父消息 ID。
    pub(crate) fn with_parent_id(mut self, parent_id: UiMessageId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// 绑定会话 ID。
    pub(crate) fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 绑定创建时间戳。
    pub(crate) fn with_created_ms(mut self, created_ms: u64) -> Self {
        self.created_ms = Some(created_ms);
        self
    }

    /// 标记消息来自 gateway、命令或本地恢复流程。
    pub(crate) fn with_origin(mut self, origin: UiMessageOrigin) -> Self {
        self.origin = Some(origin);
        self
    }
}

/// 消息进入 UI 管线的来源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiMessageOrigin {
    Gateway { event_type: String },
    Command { name: String },
    Local,
}

/// 新 TUI 内部使用的 token 统计结构。
///
/// 这里刻意复制共享层字段，而不是直接把共享结构塞进 view model，避免后续 renderer
/// 与 selector 被共享层序列化细节反向牵引。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UiTokenUsage {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cached_tokens: i64,
    pub(crate) reasoning_tokens: i64,
}

impl From<session_ui::TokenUsage> for UiTokenUsage {
    fn from(value: session_ui::TokenUsage) -> Self {
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            cached_tokens: value.cached_tokens,
            reasoning_tokens: value.reasoning_tokens,
        }
    }
}

impl From<&session_ui::TokenUsage> for UiTokenUsage {
    fn from(value: &session_ui::TokenUsage) -> Self {
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
            cached_tokens: value.cached_tokens,
            reasoning_tokens: value.reasoning_tokens,
        }
    }
}

/// thinking 块的时间片视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiThinkingTiming {
    pub(crate) start_ms: u64,
    pub(crate) end_ms: Option<u64>,
    pub(crate) last_update_ms: u64,
}

impl From<session_ui::ThinkTiming> for UiThinkingTiming {
    fn from(value: session_ui::ThinkTiming) -> Self {
        Self {
            start_ms: value.start_ms,
            end_ms: value.end_ms,
            last_update_ms: value.last_update_ms,
        }
    }
}

impl From<&session_ui::ThinkTiming> for UiThinkingTiming {
    fn from(value: &session_ui::ThinkTiming) -> Self {
        Self {
            start_ms: value.start_ms,
            end_ms: value.end_ms,
            last_update_ms: value.last_update_ms,
        }
    }
}

/// 一次 turn 的终态语义。
///
/// 该枚举刻意与当前 legacy/gateway 对照桥的 done/cancelled/timeout/error 终态对齐，
/// 方便后续 reducer 在不感知底层 transport 来源的情况下处理 turn 生命周期。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum UiTurnTerminal {
    #[default]
    Pending,
    Streaming,
    Done {
        finish_reason: Option<String>,
    },
    Cancelled {
        reason: Option<String>,
    },
    TimedOut {
        message: String,
    },
    Error {
        message: String,
    },
}

/// 工具调用消息的执行状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum UiToolCallState {
    #[default]
    Queued,
    Running,
    Complete,
    Failed,
    Cancelled,
}

/// step 消息的执行状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum UiStepState {
    #[default]
    Pending,
    Running,
    Complete,
    Cancelled,
    Failed,
}

/// system 消息的展示级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum UiSystemMessageLevel {
    #[default]
    Info,
    Warning,
    Success,
}

/// 用户消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiUserMessage {
    pub(crate) base: UiMessageBase,
    pub(crate) text: String,
}

/// 助手文本消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiAssistantMessage {
    pub(crate) base: UiMessageBase,
    pub(crate) text: String,
    pub(crate) usage: UiTokenUsage,
    pub(crate) step_count: usize,
    pub(crate) terminal: UiTurnTerminal,
    pub(crate) model: Option<String>,
}

/// 工具调用消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiToolCall {
    pub(crate) base: UiMessageBase,
    pub(crate) call_id: Option<String>,
    pub(crate) tool_name: String,
    pub(crate) summary: Option<String>,
    pub(crate) arguments: Option<String>,
    pub(crate) state: UiToolCallState,
}

/// 工具结果消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiToolResult {
    pub(crate) base: UiMessageBase,
    pub(crate) call_id: Option<String>,
    pub(crate) tool_name: String,
    pub(crate) command: Option<String>,
    pub(crate) content: String,
    pub(crate) is_error: bool,
}

/// thinking 内容块。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum UiThinkingPhase {
    #[default]
    Streaming,
    Closed,
    Redacted,
    HiddenHistory,
}

/// thinking 内容块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiThinkingBlock {
    pub(crate) base: UiMessageBase,
    pub(crate) summary: Option<String>,
    pub(crate) content: String,
    pub(crate) timing: Vec<UiThinkingTiming>,
    pub(crate) collapsed: bool,
    pub(crate) phase: UiThinkingPhase,
}

/// step 行为消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiStep {
    pub(crate) base: UiMessageBase,
    pub(crate) step_index: u32,
    pub(crate) started_ms: u64,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) usage: UiTokenUsage,
    pub(crate) finish_reason: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) state: UiStepState,
}

/// 系统提示消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiSystemMessage {
    pub(crate) base: UiMessageBase,
    pub(crate) text: String,
    pub(crate) level: UiSystemMessageLevel,
}

/// 错误消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiErrorMessage {
    pub(crate) base: UiMessageBase,
    pub(crate) message: String,
    pub(crate) recoverable: bool,
}

/// 可更新的后台进度消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiProgressMessage {
    pub(crate) base: UiMessageBase,
    pub(crate) label: String,
    pub(crate) current: Option<u64>,
    pub(crate) total: Option<u64>,
    pub(crate) completed: bool,
}

/// 会话压缩前后之间的稳定 transcript 边界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiCompactBoundary {
    pub(crate) base: UiMessageBase,
    pub(crate) summary: String,
    pub(crate) compacted_messages: usize,
}

/// 未识别外部内容块的显式、安全降级。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiUnknownMessage {
    pub(crate) base: UiMessageBase,
    pub(crate) external_kind: String,
    pub(crate) summary: String,
}

/// 新 TUI 的主消息枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiMessage {
    User(UiUserMessage),
    Assistant(UiAssistantMessage),
    ToolCall(UiToolCall),
    ToolResult(UiToolResult),
    Thinking(UiThinkingBlock),
    Step(UiStep),
    System(UiSystemMessage),
    Error(UiErrorMessage),
    Attachment(UiAttachment),
    Progress(UiProgressMessage),
    CompactBoundary(UiCompactBoundary),
    Unknown(UiUnknownMessage),
}

/// `UiMessage` 的逻辑分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiMessageKind {
    User,
    Assistant,
    ToolCall,
    ToolResult,
    Thinking,
    Step,
    System,
    Error,
    Attachment,
    Progress,
    CompactBoundary,
    Unknown,
}

impl UiMessage {
    /// 返回消息的共享元信息。
    pub(crate) fn base(&self) -> &UiMessageBase {
        match self {
            Self::User(message) => &message.base,
            Self::Assistant(message) => &message.base,
            Self::ToolCall(message) => &message.base,
            Self::ToolResult(message) => &message.base,
            Self::Thinking(message) => &message.base,
            Self::Step(message) => &message.base,
            Self::System(message) => &message.base,
            Self::Error(message) => &message.base,
            Self::Attachment(message) => &message.base,
            Self::Progress(message) => &message.base,
            Self::CompactBoundary(message) => &message.base,
            Self::Unknown(message) => &message.base,
        }
    }

    /// 返回消息逻辑分类。
    pub(crate) fn kind(&self) -> UiMessageKind {
        match self {
            Self::User(_) => UiMessageKind::User,
            Self::Assistant(_) => UiMessageKind::Assistant,
            Self::ToolCall(_) => UiMessageKind::ToolCall,
            Self::ToolResult(_) => UiMessageKind::ToolResult,
            Self::Thinking(_) => UiMessageKind::Thinking,
            Self::Step(_) => UiMessageKind::Step,
            Self::System(_) => UiMessageKind::System,
            Self::Error(_) => UiMessageKind::Error,
            Self::Attachment(_) => UiMessageKind::Attachment,
            Self::Progress(_) => UiMessageKind::Progress,
            Self::CompactBoundary(_) => UiMessageKind::CompactBoundary,
            Self::Unknown(_) => UiMessageKind::Unknown,
        }
    }

    /// 返回内部消息 ID。
    pub(crate) fn id(&self) -> &UiMessageId {
        &self.base().id
    }
}
