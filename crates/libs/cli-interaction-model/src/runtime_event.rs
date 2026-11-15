//! 交互状态消费的运行事件数据；协议适配由运行层负责。

use vw_gateway_client::GatewayChatUsage;

use crate::UiMessage;

/// CLI 新 TUI 在 runtime 边界统一消费的终态事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiRuntimeTerminalEvent {
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

/// CLI 新 TUI 在 runtime 边界统一消费的事件枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiRuntimeEvent {
    /// 后台计划运行已分配任务 ID，可用于停止控制面。
    PlanRunTaskStarted { task_id: String },
    /// 助手文本增量。
    Delta(String),
    /// 本轮实际加载的上下文指令文件。
    ContextLoaded { paths: Vec<String> },
    /// gateway 已标准化的非文本内容块。
    Message(UiMessage),
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
    /// 当前工具轮次已完成，下一次模型请求尚未开始。
    PostToolRound,
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
