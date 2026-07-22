//! gateway stream 到 runtime 事件的适配层。
//!
//! 本模块负责两类事情：
//! - 将 gateway client 暴露的 typed stream 事件转换为 CLI 内部事件
//! - 统一终态控制语义，把 done/error/cancel/timeout 收口为稳定枚举

use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatUsage, GatewayTypedChatStreamEvent,
    normalize_chat_stream_event,
};

/// CLI 新 TUI 在 runtime 边界统一消费的终态事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiRuntimeTerminalEvent {
    /// 当前 chat stream 正常结束。
    Done {
        finish_reason: Option<String>,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 当前 chat stream 被主动取消或被中断终止。
    Cancelled {
        reason: Option<String>,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 当前 chat stream 以超时结束。
    TimedOut {
        message: String,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 当前 chat stream 以错误结束。
    Error(String),
}

impl UiRuntimeTerminalEvent {
    /// 基于错误文本归类统一终态语义。
    pub(crate) fn from_error_message(error: String) -> Self {
        let message = normalize_optional_string(Some(error))
            .unwrap_or_else(|| "gateway stream failed".to_string());

        match classify_terminal_marker(Some(&message)) {
            TerminalMarker::Done => Self::Error(message),
            TerminalMarker::Cancelled => Self::Cancelled {
                reason: Some(message),
                usage: None,
                message_id: None,
                parent_message_id: None,
            },
            TerminalMarker::TimedOut => {
                Self::TimedOut { message, usage: None, message_id: None, parent_message_id: None }
            }
        }
    }
}

/// CLI 新 TUI 在 runtime 边界统一消费的事件枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UiRuntimeEvent {
    /// 助手文本增量。
    Delta(String),
    /// 一个新的推理步骤开始执行。
    StepStart { step_index: u32, created_ms: u64, model: Option<String> },
    /// 一个推理步骤执行完成。
    StepFinish {
        step_index: u32,
        finished_ms: u64,
        usage: GatewayChatUsage,
        finish_reason: Option<String>,
        model: Option<String>,
    },
    /// 当前 chat stream 的终态控制事件。
    Terminal(UiRuntimeTerminalEvent),
    /// 会话任务侧数据已变化，需从 runtime 重新同步 question/todo 状态。
    TaskStateChanged { session_id: Option<String> },
    /// 会话元数据已变化，需刷新标题、scope、path 或 preview。
    SessionMetadataChanged { session_id: Option<String>, title: Option<String> },
    /// 用量快照已变化；当前先显式识别，避免误记为 unknown warning。
    UsageUpdated { session_id: Option<String>, usage: GatewayChatUsage },
    /// 当前 runtime 尚未识别的事件类型。
    Unknown { event_type: Option<String> },
}

/// 将 gateway 原始流事件规整为 CLI 内部事件。
pub(crate) fn adapt_gateway_stream_event(event: GatewayChatStreamEvent) -> UiRuntimeEvent {
    match normalize_chat_stream_event(event) {
        GatewayTypedChatStreamEvent::Delta(delta) => UiRuntimeEvent::Delta(delta),
        GatewayTypedChatStreamEvent::StepStart(event) => UiRuntimeEvent::StepStart {
            step_index: event.step_index,
            created_ms: event.created_ms,
            model: event.model,
        },
        GatewayTypedChatStreamEvent::StepFinish(event) => UiRuntimeEvent::StepFinish {
            step_index: event.step_index,
            finished_ms: event.finished_ms,
            usage: event.usage,
            finish_reason: event.finish_reason,
            model: event.model,
        },
        GatewayTypedChatStreamEvent::PostToolRound(_) => {
            UiRuntimeEvent::Unknown { event_type: Some("chat.post_tool_round".to_string()) }
        }
        GatewayTypedChatStreamEvent::Done {
            finish_reason,
            usage,
            message_id,
            parent_message_id,
        } => UiRuntimeEvent::Terminal(terminal_from_done(
            finish_reason,
            usage,
            message_id,
            parent_message_id,
        )),
        GatewayTypedChatStreamEvent::Error(error) => {
            UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::from_error_message(error))
        }
        GatewayTypedChatStreamEvent::TodoUpdated { session_id }
        | GatewayTypedChatStreamEvent::QuestionRaised { session_id }
        | GatewayTypedChatStreamEvent::QuestionResolved { session_id } => {
            UiRuntimeEvent::TaskStateChanged { session_id }
        }
        GatewayTypedChatStreamEvent::TitleUpdated { session_id, title } => {
            UiRuntimeEvent::SessionMetadataChanged {
                session_id,
                title: normalize_optional_string(Some(title)),
            }
        }
        GatewayTypedChatStreamEvent::SessionUpdated { session_id, title } => {
            UiRuntimeEvent::SessionMetadataChanged {
                session_id,
                title: normalize_optional_string(title),
            }
        }
        GatewayTypedChatStreamEvent::UsageUpdated { session_id, usage } => {
            UiRuntimeEvent::UsageUpdated { session_id, usage }
        }
        GatewayTypedChatStreamEvent::Unknown { event_type } => {
            UiRuntimeEvent::Unknown { event_type }
        }
    }
}

/// 将 `chat.done` 事件规整为统一终态事件。
fn terminal_from_done(
    finish_reason: Option<String>,
    usage: Option<GatewayChatUsage>,
    message_id: Option<String>,
    parent_message_id: Option<String>,
) -> UiRuntimeTerminalEvent {
    let finish_reason = normalize_optional_string(finish_reason);
    match classify_terminal_marker(finish_reason.as_deref()) {
        TerminalMarker::Done => {
            UiRuntimeTerminalEvent::Done { finish_reason, usage, message_id, parent_message_id }
        }
        TerminalMarker::Cancelled => UiRuntimeTerminalEvent::Cancelled {
            reason: finish_reason,
            usage,
            message_id,
            parent_message_id,
        },
        TerminalMarker::TimedOut => UiRuntimeTerminalEvent::TimedOut {
            message: finish_reason
                .clone()
                .unwrap_or_else(|| "gateway stream timed out".to_string()),
            usage,
            message_id,
            parent_message_id,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalMarker {
    Done,
    Cancelled,
    TimedOut,
}

/// 基于 finish_reason 或错误文本，归类统一的终态语义。
fn classify_terminal_marker(value: Option<&str>) -> TerminalMarker {
    let Some(value) = normalize_optional_str_ref(value) else {
        return TerminalMarker::Done;
    };

    let normalized = value.to_ascii_lowercase();
    if contains_terminal_keyword(&normalized, &["timeout", "timed out", "deadline exceeded"]) {
        return TerminalMarker::TimedOut;
    }

    if contains_terminal_keyword(
        &normalized,
        &["cancelled", "canceled", "cancel", "interrupted", "aborted"],
    ) {
        return TerminalMarker::Cancelled;
    }

    TerminalMarker::Done
}

/// 将外部传入的字符串归一化为“空白即无值”。
fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

/// 将可选字符串引用归一化为“空白即无值”。
fn normalize_optional_str_ref(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// 检查标准化文本中是否包含任一终态关键字。
pub(crate) fn contains_terminal_keyword(value: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| value.contains(keyword))
}
