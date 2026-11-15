//! 工具循环错误类型定义模块
//!
//! 本模块提供了与工具循环（tool loop）相关的错误类型和辅助函数。
//! 主要用于标识和处理工具循环执行过程中的异常情况。
//!
//! # 主要功能
//!
//! - 定义工具循环取消错误类型
//! - 提供错误类型识别的辅助函数
//! - 支持工具迭代次数超限错误的检测

/// 工具循环取消错误
///
/// 当工具循环被显式取消时使用此错误类型。
/// 这是一个标记性错误类型，用于在错误链中识别取消操作。
///
/// # 示例
///
/// ```ignore
/// use agent_loop::core::errors::ToolLoopCancelled;
///
/// let error = anyhow::Error::new(ToolLoopCancelled);
/// assert!(is_tool_loop_cancelled(&error));
/// ```
#[derive(Debug)]
pub struct ToolLoopCancelled;

/// 为 ToolLoopCancelled 实现 Display trait
///
/// 提供人类可读的错误描述信息。
impl std::fmt::Display for ToolLoopCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("tool loop cancelled")
    }
}

/// 为 ToolLoopCancelled 实现 Error trait
///
/// 使该类型可以作为标准错误类型使用，支持错误链追踪。
impl std::error::Error for ToolLoopCancelled {}

/// 检查错误链中是否包含工具循环取消错误
///
/// 遍历错误链，检查是否存在 ToolLoopCancelled 类型的错误。
///
/// # 参数
///
/// - `err`: 需要检查的 anyhow::Error 引用
///
/// # 返回值
///
/// 如果错误链中包含 ToolLoopCancelled 错误，返回 `true`；否则返回 `false`。
///
/// # 示例
///
/// ```ignore
/// use anyhow::anyhow;
/// use agent_loop::core::errors::{ToolLoopCancelled, is_tool_loop_cancelled};
///
/// let error = anyhow!(ToolLoopCancelled);
/// assert!(is_tool_loop_cancelled(&error));
///
/// let other_error = anyhow!("some other error");
/// assert!(!is_tool_loop_cancelled(&other_error));
/// ```
pub fn is_tool_loop_cancelled(err: &anyhow::Error) -> bool {
    err.chain().any(|source| source.is::<ToolLoopCancelled>())
}

/// 检查错误是否为工具迭代次数超限错误
///
/// 通过检查错误消息内容，判断是否为代理超过最大工具迭代次数的错误。
///
/// # 参数
///
/// - `err`: 需要检查的 anyhow::Error 引用
///
/// # 返回值
///
/// 如果错误消息中包含 "Agent exceeded maximum tool iterations"，返回 `true`；否则返回 `false`。
///
/// # 示例
///
/// ```ignore
/// use anyhow::anyhow;
/// use agent_loop::core::errors::is_tool_iteration_limit_error;
///
/// let error = anyhow!("Agent exceeded maximum tool iterations (100)");
/// assert!(is_tool_iteration_limit_error(&error));
///
/// let other_error = anyhow!("some other error");
/// assert!(!is_tool_iteration_limit_error(&other_error));
/// ```
pub fn is_tool_iteration_limit_error(err: &anyhow::Error) -> bool {
    err.chain().any(|source| source.to_string().contains("Agent exceeded maximum tool iterations"))
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;
