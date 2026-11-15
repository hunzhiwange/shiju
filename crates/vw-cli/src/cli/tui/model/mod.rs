//! TUI 内部模型与编辑状态层。
//!
//! 本模块定义内部 view model 及其内聚的 Unicode 编辑操作，不承担 reducer、renderer 或 gateway
//! 调度职责。当前阶段先收口三类核心模型：
//! - `ui_message`: 用户/助手/tool/thinking/step/system/error 等消息结构
//! - `prompt`: 输入框、历史、排队命令与一次提交的生命周期
//! - `overlay`: confirm/search/question/todo/task/palette/error 的弹层状态
//!
//! `editor`、`paste`、`vim` 与 `prompt` 同属本边界，不反向依赖交互运行层。

pub(crate) mod agent_view;
pub(crate) mod attachment;
pub(crate) mod background_task;
pub(crate) mod editor;
pub(crate) mod elicitation;
pub(crate) mod integration;
pub(crate) mod message_action;
pub(crate) mod notification;
pub(crate) mod overlay;
pub(crate) mod paste;
pub(crate) mod permission;
pub(crate) mod prompt;
pub(crate) mod schedule_panel;
pub(crate) mod search;
pub(crate) mod session_mode;
pub(crate) mod settings_panel;
pub(crate) mod transcript_mode;
pub(crate) mod ui_message;
pub(crate) mod vim;
pub(crate) mod workflow_panel;

#[allow(unused_imports)]
pub(crate) use attachment::{UiAttachment, UiAttachmentKind};
#[allow(unused_imports)]
pub(crate) use editor::PromptMotion;
#[allow(unused_imports)]
pub(crate) use message_action::{MessageActionIntent, MessageActionKind, MessageActionState};
#[allow(unused_imports)]
pub(crate) use overlay::{
    McpServerTransport, OverlayFocus, OverlayState, UiConfirmIntent, UiConfirmOverlay,
    UiErrorOverlay, UiMcpOverlay, UiMcpServerInfo, UiMemoryEntry, UiMemoryOverlay, UiOverlay,
    UiOverlayKind, UiPermissionOverlay, UiQuestionOverlay, UiQuestionSurfaceKind, UiReportOverlay,
    UiSearchMatch, UiSearchOverlay, UiTaskOverlay, UiTaskStepItem, UiTodoOverlay,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use overlay::{UiQuestionOption, UiQuestionPrompt, UiQuestionToolContext, UiTodoItem};
#[allow(unused_imports)]
pub(crate) use permission::PermissionDecision;
#[allow(unused_imports)]
pub(crate) use prompt::{
    PromptMode, PromptState, PromptSubmission, PromptSubmissionStatus,
    QueueDrainBoundary, QueuedPromptCommand, QueuedPromptCommandKind, QueuedPromptPriority,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use session_mode::GoalMode;
#[allow(unused_imports)]
pub(crate) use session_mode::{
    AcpBackendOption, DelegateAgentOption, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix, SessionOptions, WorkflowMode,
};
#[allow(unused_imports)]
pub(crate) use transcript_mode::{
    TranscriptEffectIntent, TranscriptModeState, TranscriptScrollRestore,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use ui_message::UiMessageKind;
#[allow(unused_imports)]
pub(crate) use ui_message::{
    UiAssistantMessage, UiCompactBoundary, UiErrorMessage, UiMessage, UiMessageBase, UiMessageId,
    UiMessageOrigin, UiProgressMessage, UiStep, UiStepState, UiSystemMessage, UiSystemMessageLevel,
    UiThinkingBlock, UiThinkingPhase, UiThinkingTiming, UiTokenUsage, UiToolCall, UiToolCallState,
    UiToolResult, UiTurnTerminal, UiUnknownMessage, UiUserMessage,
};
