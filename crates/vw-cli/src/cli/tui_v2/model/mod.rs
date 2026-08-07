//! TUI v2 内部模型层。
//!
//! 本模块只定义 CLI 新 TUI 的内部 view model，不承担 reducer、renderer 或 gateway
//! 调度职责。当前阶段先收口三类核心模型：
//! - `ui_message`: 用户/助手/tool/thinking/step/system/error 等消息结构
//! - `prompt`: 输入框、历史、排队命令与一次提交的生命周期
//! - `overlay`: confirm/search/question/todo/task/palette/error 的弹层状态
//!
//! 这些结构的目标是为后续 S2-2 的 reducer/store 与 S4-1 的事件标准化提供稳定承接面。

pub(crate) mod agent_view;
pub(crate) mod attachment;
pub(crate) mod background_task;
pub(crate) mod elicitation;
pub(crate) mod integration;
pub(crate) mod message_action;
pub(crate) mod notification;
pub(crate) mod overlay;
#[cfg(test)]
#[path = "overlay_tests.rs"]
mod overlay_tests;
pub(crate) mod permission;
pub(crate) mod prompt;
#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
pub(crate) mod search;
pub(crate) mod schedule_panel;
pub(crate) mod session_mode;
pub(crate) mod settings_panel;
pub(crate) mod transcript_mode;
pub(crate) mod ui_message;
pub(crate) mod workflow_panel;
#[cfg(test)]
#[path = "ui_message_tests.rs"]
mod ui_message_tests;

pub(crate) use attachment::{UiAttachment, UiAttachmentKind};
pub(crate) use message_action::{MessageActionIntent, MessageActionKind, MessageActionState};
pub(crate) use overlay::{
    McpServerTransport, OverlayFocus, OverlayState, UiConfirmIntent, UiConfirmOverlay,
    UiErrorOverlay, UiMcpOverlay, UiMcpServerInfo, UiMemoryEntry, UiMemoryOverlay, UiOverlay,
    UiOverlayKind, UiPermissionOverlay, UiQuestionOverlay, UiQuestionSurfaceKind, UiReportOverlay,
    UiSearchMatch, UiSearchOverlay, UiTaskOverlay, UiTaskStepItem, UiTodoOverlay,
};
#[cfg(test)]
pub(crate) use overlay::{UiQuestionOption, UiQuestionPrompt, UiQuestionToolContext, UiTodoItem};
pub(crate) use permission::PermissionDecision;
pub(crate) use prompt::{
    PromptMode, PromptMotion, PromptState, PromptSubmission, PromptSubmissionStatus,
    QueueDrainBoundary, QueuedPromptCommand, QueuedPromptCommandKind, QueuedPromptPriority,
};
pub(crate) use session_mode::{
    AcpBackendOption, DelegateAgentOption, GoalMode, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix, SessionOptions, WorkflowMode,
};
pub(crate) use transcript_mode::{
    TranscriptEffectIntent, TranscriptModeState, TranscriptScrollRestore,
};
#[cfg(test)]
pub(crate) use ui_message::UiMessageKind;
pub(crate) use ui_message::{
    UiAssistantMessage, UiCompactBoundary, UiErrorMessage, UiMessage, UiMessageBase, UiMessageId,
    UiMessageOrigin, UiProgressMessage, UiStep, UiStepState, UiSystemMessage, UiSystemMessageLevel,
    UiThinkingBlock, UiThinkingPhase, UiThinkingTiming, UiTokenUsage, UiToolCall, UiToolCallState,
    UiToolResult, UiTurnTerminal, UiUnknownMessage, UiUserMessage,
};

#[cfg(test)]
mod tests;
