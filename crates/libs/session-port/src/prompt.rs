//! 协议中立的提示输出。

use crate::{CompactionPhase, ModelRecovery, ToolEvent};
use crate::llm_raw::LlmRawArtifact;
use session_ui_types::TokenUsage;

/// 单轮提示结束原因。
#[derive(Debug, Clone)]
pub enum PromptTermination {
    /// 处理器正常完成并返回总令牌用量。
    Completed(TokenUsage),
    /// 调用方显式取消了活动轮次。
    Cancelled,
    /// 处理器以错误终止。
    Failed(String),
}

/// 共享提示运行时输出的协议中立事件。
#[derive(Debug, Clone)]
pub enum PromptEvent {
    /// 仅供 Gateway 暂存的原始 LLM 工件。
    LlmRawArtifact(LlmRawArtifact),

    /// 实际加载的上下文文件。
    ContextLoaded {
        /// 文件路径列表。
        paths: Vec<String>,
    },
    /// 可恢复模型故障的非终止状态。
    ModelRecovery(ModelRecovery),
    /// 助手文本增量。
    Text(String),
    /// 已净化的类型化工具事件。
    Tool(ToolEvent),
    /// 未完成待办触发自动续轮。
    TodoContinuation,
    /// 自动压缩生命周期事件。
    Compaction {
        /// 当前压缩阶段。
        phase: CompactionPhase,
    },
    /// 处理器步骤开始。
    StepStart {
        /// 步骤索引。
        step_index: u32,
        /// 创建时间。
        created_ms: u64,
        /// 模型。
        model: Option<String>,
    },
    /// 处理器步骤完成。
    StepFinish {
        /// 步骤索引。
        step_index: u32,
        /// 完成时间。
        finished_ms: u64,
        /// 用量。
        usage: TokenUsage,
        /// 完成原因。
        finish_reason: Option<String>,
        /// 模型。
        model: Option<String>,
    },
    /// 工具轮次结束。
    PostToolRound {
        /// 步骤索引。
        step_index: u32,
    },
    /// 单轮唯一终态。
    Terminated(PromptTermination),
}
