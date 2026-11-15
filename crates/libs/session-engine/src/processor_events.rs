//! Shared processor events; request startup remains host-owned.
use session_port::{ModelRecovery, CompactionPhase, ToolEvent};
use session_port::llm_raw::LlmRawArtifact;
use session_port::ui_types as models;

/// 工具调用结构
///
/// 该结构体表示 LLM 请求执行的工具调用，包含工具调用的所有必要信息。
/// 在函数调用场景中，LLM 会返回此类结构以请求执行特定工具。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolCall {
    pub kind: model_types::ToolCallKind,
    pub namespace: Option<String>,
    /// 工具调用的唯一标识符，用于关联调用和结果
    pub id: String,
    /// 要调用的工具名称
    pub name: String,
    /// 工具参数的 JSON 字符串表示
    pub arguments: String,
}

/// 流式事件枚举
///
/// 定义了会话处理过程中可能产生的各种流式事件类型。
/// 这些事件用于实时通知客户端会话处理的进度和状态变化。
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 独立于最终错误和模型文本的恢复进度。
    ModelRecovery(ModelRecovery),
    /// 仅供 Gateway 暂存、不会编码给客户端的原始 LLM 工件。
    LlmRawArtifact(LlmRawArtifact),

    /// 系统提示词已加载协议文件，客户端可据此展示真实的上下文读取记录。
    ContextLoaded {
        /// 实际读取并注入系统提示词的协议文件路径。
        paths: Vec<String>,
    },

    /// 增量文本事件，包含新生成的文本片段
    Delta(String),

    /// Model calls before local execution, retaining their protocol identity.
    ModelToolCalls(Vec<ToolCall>),

    /// 类型化工具生命周期事件。
    Tool(ToolEvent),

    /// 未完成待办触发自动续轮，独立于模型正文。
    TodoContinuation,

    /// 自动压缩生命周期事件。
    Compaction {
        /// 当前压缩阶段。
        phase: CompactionPhase,
    },

    /// 步骤开始事件，表示一个新的处理步骤开始执行
    StepStart {
        /// 步骤索引，从 0 开始递增
        step_index: u32,

        /// 步骤创建时间戳（毫秒级）
        created_ms: u64,

        /// 当前步骤使用的模型标识符
        model: Option<String>,
    },

    /// 步骤完成事件，表示一个处理步骤执行完毕
    StepFinish {
        /// 步骤索引，与 StepStart 中的 step_index 对应
        step_index: u32,

        /// 步骤完成时间戳（毫秒级）
        finished_ms: u64,

        /// 当前步骤的令牌使用统计
        usage: models::TokenUsage,

        /// 完成原因（如 "stop"、"length"、"content_filter" 等）
        finish_reason: Option<String>,

        /// 完成该步骤时使用的模型标识符
        model: Option<String>,
    },

    /// 工具轮次完成事件，表示本轮工具执行已结束，下一轮模型请求尚未开始
    PostToolRound {
        /// 对应的步骤索引
        step_index: u32,
    },

    /// 全部完成事件，表示整个会话处理结束
    Done(models::TokenUsage),

    /// 错误事件，表示处理过程中发生了错误
    Error(String),
}
