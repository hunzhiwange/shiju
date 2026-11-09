//! 消息执行结果模块
//!
//! 该模块定义了 LLM 执行结果的表示方式，以及基于执行结果生成用户反馈的工具函数。
//! 主要用于通道管理器中处理和展示异步 LLM 调用的最终状态。

use super::*;

/// LLM 执行结果枚举
///
/// 表示 LLM 调用的最终执行状态，包括正常完成和用户取消两种情况。
///
/// # 变体说明
///
/// - `Completed` - LLM 调用正常完成（无论成功或失败）
///   - 外层 `Result` 表示是否超时
///   - 内层 `Result<ChannelProcessorResponse, anyhow::Error>` 表示实际的执行结果
///     - `Ok(String)` 包含 LLM 返回的文本内容
///     - `Err(anyhow::Error)` 包含执行过程中的错误信息
/// - `Cancelled` - 用户主动取消了 LLM 调用
///
/// # 示例
///
/// ```ignore
/// // 成功完成的执行
/// let result = LlmExecutionResult::Completed(Ok(Ok("LLM 响应内容".into())));
///
/// // 执行超时
/// let result = LlmExecutionResult::Completed(Err(tokio::time::error::Elapsed::new()));
///
/// // 执行出错
/// let result = LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!("执行失败"))));
///
/// // 用户取消
/// let result = LlmExecutionResult::Cancelled;
/// ```
pub(crate) enum LlmExecutionResult {
    /// 已完成的执行结果
    ///
    /// 包含一个嵌套的 Result 结构：
    /// - 外层 Result 处理超时情况
    /// - 内层 Result 处理实际的成功/失败
    Completed(Result<Result<ChannelProcessorResponse, anyhow::Error>, tokio::time::error::Elapsed>),

    /// 用户取消的执行
    Cancelled,
}

/// 根据执行结果生成反应表情符号
///
/// 该函数用于在通道中向用户展示直观的执行状态反馈。
/// 成功时显示绿色对勾，其他情况（失败、超时、取消）显示警告标志。
///
/// # 参数
///
/// - `result` - LLM 执行结果的引用
///
/// # 返回值
///
/// 返回静态字符串切片，表示对应状态的表情符号：
/// - 成功完成：`"✅"` (U+2705)
/// - 其他情况：`"⚠️"` (U+26A0 U+FE0F)
///
/// # 示例
///
/// ```ignore
/// let success = LlmExecutionResult::Completed(Ok(Ok("内容".into())));
/// assert_eq!(reaction_done_emoji(&success), "✅");
///
/// let failed = LlmExecutionResult::Completed(Ok(Err(anyhow::anyhow!("错误"))));
/// assert_eq!(reaction_done_emoji(&failed), "⚠️");
///
/// let cancelled = LlmExecutionResult::Cancelled;
/// assert_eq!(reaction_done_emoji(&cancelled), "⚠️");
/// ```
pub(crate) fn reaction_done_emoji(result: &LlmExecutionResult) -> &'static str {
    match result {
        // 执行成功完成（未超时且未出错）时显示成功标志
        LlmExecutionResult::Completed(Ok(Ok(_))) => "\u{2705}",
        // 所有其他情况（超时、错误、取消）都显示警告标志
        _ => "\u{26A0}\u{FE0F}",
    }
}

#[cfg(test)]
#[path = "message_result_tests.rs"]
mod message_result_tests;
