use serde_json::Value;
use model_types::ToolCall;
use message_types as message;
use session_ui_types as models;

/// 流式事件枚举
///
/// 该枚举定义了 LLM 流式响应中可能出现的所有事件类型。
/// 用于在流式处理过程中传递增量文本、推理内容、工具调用和完成状态。
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// The selected transport is ACP and may be silent while executing remote tools.
    AcpStarted,
    /// 文本增量事件，包含新生成的文本片段
    Delta(String),
    /// 推理增量事件，包含模型的推理过程文本（用于思维链模型）
    ReasoningDelta(String),
    /// 工具调用事件，包含需要执行的工具调用列表
    ToolCalls(Vec<ToolCall>),
    /// 远端 ACP 代理已经执行的工具生命周期更新。
    ///
    /// 该事件只用于展示，不能再次作为本地工具调用执行。
    RemoteToolUpdate {
        /// ACP 工具调用的稳定标识。
        call_id: String,
        /// 工具名称。
        name: String,
        /// 是否为该调用的初始事件。
        is_initial: bool,
        /// 已净化的结构化生命周期载荷。
        payload: Value,
    },
    /// 完成事件，包含结束原因和令牌使用统计
    Done {
        /// 完成原因（如"stop"、"length"等）
        finish_reason: Option<String>,
        /// 令牌使用统计（输入和输出令牌数）
        usage: models::TokenUsage,
    },
    /// 错误事件，包含错误详情
    Error(message::AssistantError),
    /// 完整消息事件，包含所有消息的完整列表
    FullMessages(Vec<serde_json::Value>),
}
