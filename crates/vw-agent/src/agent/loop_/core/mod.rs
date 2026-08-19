//! # 代理循环核心模块
//!
//! 本模块提供代理主循环的核心基础设施，包括工具调用循环、历史管理、
//! 超时控制和轮次编排等功能。
//!
//! ## 模块结构
//!
//! - [`constants`] - 循环相关的常量定义（最大迭代次数、历史消息限制等）
//! - [`errors`] - 循环执行过程中的错误类型定义
//! - [`history`] - 对话历史构建与记忆自动保存逻辑
//! - [`timeouts`] - 消息超时预算计算与动态调整
//! - [`tool_loop`] - 工具调用循环的核心执行逻辑
//! - [`turn`] - 代理轮次编排与 CLI/非 CLI 上下文适配
//!
//! ## 设计原则
//!
//! - 所有公共导出项均标记为 `pub(crate)`，仅限 crate 内部使用
//! - 超时与迭代限制通过常量集中管理，便于全局调优
//! - 工具循环支持取消检测与迭代上限保护，防止无限循环

mod constants;
mod errors;
mod history;
mod timeouts;
mod tool_loop;
mod turn;

// 工具循环回复目标地址的任务本地存储。
//
// 用于在异步任务树中传递回复目标地址（如 channel 消息的回复目标），
// 避免在深层调用链中逐层传递参数。
//
// 使用场景：
// - Channel 消息处理时，记录原始消息来源以便回复
// - 工具执行需要向特定目标发送通知时
//
// 使用 `tokio::task_local!` 宏定义，确保在 Tokio 任务上下文中安全访问。
// 值为 `Option<String>`，`None` 表示无特定回复目标。
tokio::task_local! {
    static TOOL_LOOP_REPLY_TARGET: Option<String>;
}

pub use errors::is_tool_iteration_limit_error;
pub(crate) use errors::{ToolLoopCancelled, is_tool_loop_cancelled};
pub use tool_loop::run_tool_call_loop;
pub(crate) use turn::agent_turn;
#[cfg(test)]
pub(crate) use turn::run_tool_call_loop_with_non_cli_approval_context;

pub use constants::AUTOSAVE_MIN_MESSAGE_CHARS;
pub use history::autosave_memory_key;
pub use timeouts::{effective_message_timeout_secs, message_timeout_budget_secs};

#[cfg(test)]
pub(crate) use constants::DEFAULT_MAX_HISTORY_MESSAGES;
#[cfg(test)]
pub(crate) use constants::DEFAULT_MAX_ITERATIONS;
#[cfg(test)]
pub(crate) use history::{
    build_native_assistant_history, build_native_assistant_history_from_parsed_calls,
    tools_to_openai_format,
};
#[cfg(test)]
pub(crate) use tool_loop::looks_like_unverified_action_completion_without_tool_call;
