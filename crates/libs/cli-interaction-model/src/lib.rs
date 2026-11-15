//! TUI 交互模型与编辑状态层。
//!
//! 本模块定义内部 view model 及其内聚的 Unicode 编辑操作，不承担 reducer、renderer 或 gateway
//! 调度职责。当前阶段先收口三类核心模型：
//! - `ui_message`: 用户/助手/tool/thinking/step/system/error 等消息结构
//! - `prompt`: 输入框、历史、排队命令与一次提交的生命周期
//! - `overlay`: confirm/search/question/todo/task/palette/error 的弹层状态
//!
//! `editor`、`paste`、`vim` 与 `prompt` 同属本边界，不反向依赖交互运行层。

pub mod agent_view;
pub mod attachment;
pub mod background_task;
pub mod editor;
pub mod elicitation;
pub mod git_workspace;
pub mod integration;
pub mod message_action;
pub mod notification;
pub mod overlay;
pub mod paste;
pub mod permission;
pub mod prompt;
pub mod runtime_event;
pub mod schedule_panel;
pub mod search;
pub mod session_mode;
pub mod settings_panel;
pub mod transcript_mode;
pub mod ui_message;
pub mod vim;
pub mod workflow_panel;

#[allow(unused_imports)]
pub use attachment::{UiAttachment, UiAttachmentKind};
#[allow(unused_imports)]
pub use editor::PromptMotion;
#[allow(unused_imports)]
pub use message_action::{MessageActionIntent, MessageActionKind, MessageActionState};
#[allow(unused_imports)]
pub use overlay::{
    McpServerTransport, OverlayFocus, OverlayState, UiConfirmIntent, UiConfirmOverlay,
    UiErrorOverlay, UiMcpOverlay, UiMcpServerInfo, UiMemoryEntry, UiMemoryOverlay, UiOverlay,
    UiOverlayKind, UiPermissionOverlay, UiQuestionOverlay, UiQuestionSurfaceKind, UiReportOverlay,
    UiSearchMatch, UiSearchOverlay, UiTaskOverlay, UiTaskStepItem, UiTodoOverlay,
};
#[allow(unused_imports)]
pub use overlay::{UiQuestionOption, UiQuestionPrompt, UiQuestionToolContext, UiTodoItem};
#[allow(unused_imports)]
pub use permission::PermissionDecision;
#[allow(unused_imports)]
pub use prompt::{
    PromptMode, PromptState, PromptSubmission, PromptSubmissionStatus,
    QueueDrainBoundary, QueuedPromptCommand, QueuedPromptCommandKind, QueuedPromptPriority,
};
#[allow(unused_imports)]
pub use session_mode::GoalMode;
#[allow(unused_imports)]
pub use session_mode::{
    AcpBackendOption, DelegateAgentOption, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix, SessionOptions, WorkflowMode,
};
#[allow(unused_imports)]
pub use transcript_mode::{
    TranscriptEffectIntent, TranscriptModeState, TranscriptScrollRestore,
};
#[allow(unused_imports)]
pub use ui_message::UiMessageKind;
#[allow(unused_imports)]
pub use ui_message::{
    UiAssistantMessage, UiCompactBoundary, UiErrorMessage, UiMessage, UiMessageBase, UiMessageId,
    UiMessageOrigin, UiProgressMessage, UiStep, UiStepState, UiSystemMessage, UiSystemMessageLevel,
    UiThinkingBlock, UiThinkingPhase, UiThinkingTiming, UiTokenUsage, UiToolCall, UiToolCallState,
    UiToolResult, UiTurnTerminal, UiUnknownMessage, UiUserMessage,
};

pub use git_workspace::GitWorkspaceStatus;
pub use runtime_event::{UiRuntimeEvent, UiRuntimeTerminalEvent};

#[cfg(test)]
mod git_workspace_tests;

pub mod snapshots;
