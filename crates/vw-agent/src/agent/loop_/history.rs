//! 对话历史管理模块
//!
//! 本模块提供对话历史的裁剪、压缩和自动管理功能，用于防止对话历史无限制增长。
//! 主要包含以下功能：
//!
//! - **历史裁剪**：保留系统提示和最近的消息，移除过时的消息
//! - **历史压缩**：使用 LLM 将旧消息总结为简短的上下文摘要
//! - **自动压缩**：当历史超过阈值时自动触发压缩操作
//!
//! # 设计原则
//!
//! - 始终保留系统提示消息（如果存在）
//! - 优先保留最近的消息（更相关的上下文）
//! - 使用 LLM 智能总结以保留关键信息
//! - 摘要失败时保留原始历史

use crate::app::agent::providers::ChatMessage;
use anyhow::Result;

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
#[cfg(test)]
use std::fmt::Write;

/// 裁剪对话历史以防止无限制增长
///
/// 此函数会保留系统提示消息（如果第一条消息的角色是 "system"），
/// 并移除超出限制的旧消息。裁剪从系统提示之后开始（如果存在系统提示），
/// 确保系统提示始终保留在历史开头。
///
/// # 参数
///
/// - `history`: 待裁剪的对话历史向量，将被原地修改
/// - `max_history`: 最大允许的非系统消息数量
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::providers::ChatMessage;
///
/// let mut history = vec![
///     ChatMessage::system("You are a helpful assistant."),
///     ChatMessage::user("Hello"),
///     ChatMessage::assistant("Hi there!"),
///     // ... 更多消息
/// ];
///
/// // 只保留最近的 10 条非系统消息
/// trim_history(&mut history, 10);
/// ```
///
/// # 裁剪策略
///
/// 1. 检查第一条消息是否为系统提示
/// 2. 计算非系统消息数量
/// 3. 如果超出限制，从系统提示之后开始移除最旧的消息
pub(crate) fn trim_history(history: &mut Vec<ChatMessage>, max_history: usize) {
    // 检查第一条消息是否为系统提示
    let has_system = history.first().map_or(false, |m| m.role == "system");

    // 计算非系统消息的数量
    let non_system_count = if has_system { history.len() - 1 } else { history.len() };

    // 如果非系统消息数量在限制内，无需裁剪
    if non_system_count <= max_history {
        return;
    }

    // 确定裁剪起始位置（跳过系统提示）
    let start = if has_system { 1 } else { 0 };

    // 计算需要移除的消息数量
    let to_remove = non_system_count - max_history;

    // 移除最旧的非系统消息
    history.drain(start..start + to_remove);
}

/// 构建用于压缩的对话文本
///
/// 将消息数组转换为格式化的文本格式，便于传递给 LLM 进行总结。
/// 每条消息以 "角色: 内容" 的格式呈现，角色名称被转换为大写。
///
/// # 参数
///
/// - `messages`: 待转换的聊天消息切片
///
/// # 返回值
///
/// 返回完整对话文本，供测试核验源文本是否完整。
///
/// # 格式示例
///
/// ```text
/// USER: 你好，请帮我分析这段代码
/// ASSISTANT: 好的，我来帮你分析...
/// USER: 那性能优化怎么做？
/// ASSISTANT: 关于性能优化，我建议...
/// ```
#[cfg(test)]
pub(crate) fn build_compaction_transcript(messages: &[ChatMessage]) -> String {
    let mut transcript = String::new();

    // 将每条消息转换为 "角色: 内容" 格式
    for msg in messages {
        let role = msg.role.to_uppercase();
        let _ = writeln!(transcript, "{role}: {}", msg.content.trim());
    }

    transcript
}

/// 应用压缩摘要到对话历史
///
/// 将指定范围内的旧消息替换为压缩后的摘要消息。
/// 摘要以助手消息的形式插入，并带有 "[Compaction summary]" 标记。
///
/// # 参数
///
/// - `history`: 待修改的对话历史向量
/// - `start`: 要替换的消息范围起始索引
/// - `compact_end`: 要替换的消息范围结束索引（不包含）
/// - `summary`: 压缩后的摘要文本
///
/// # 行为说明
///
/// 此函数使用 `splice` 方法原子性地替换指定范围的消息，
/// 确保历史记录的一致性。摘要消息会被格式化为：
///
/// ```text
/// [Compaction summary]
/// - 要点 1
/// - 要点 2
/// ...
/// ```
pub(crate) fn apply_compaction_summary(
    history: &mut Vec<ChatMessage>,
    start: usize,
    compact_end: usize,
    summary: &str,
) {
    // 创建带有标记的摘要消息
    let summary_msg = ChatMessage::assistant(format!("[Compaction summary]\n{}", summary.trim()));

    // 原子性替换指定范围的消息为摘要
    history.splice(start..compact_end, std::iter::once(summary_msg));
}

/// Summarize history before the latest user request after the caller decides to compact.
/// The current request and its tool round stay intact; failed summaries never mutate history.
pub(crate) async fn compact_history_with_internal_service(
    history: &mut Vec<ChatMessage>,
    session_id: &str,
    model: &str,
    reliability: &crate::config::ReliabilityConfig,
    model_limit: &crate::provider::provider::ModelLimit,
) -> Result<bool> {
    let start = usize::from(history.first().is_some_and(|message| message.role == "system"));
    let end = history.iter().rposition(|message| message.role == "user").unwrap_or(start);
    if end <= start {
        return Ok(false);
    }
    let source = history[start..end]
        .iter()
        .map(|message| serde_json::json!({"role": message.role, "content": message.content}))
        .collect::<Vec<_>>();
    let context = crate::session::compaction_checkpoint::restore(session_id, &source)
        .await
        .map_err(anyhow::Error::msg)?;
    let summary = crate::session::compaction_summary::summarize_async(
        session_id.to_string(),
        context,
        model.to_string(),
        reliability.clone(),
        model_limit.clone(),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    crate::session::compaction_checkpoint::save(session_id, &source, &summary, None)
        .await
        .map_err(anyhow::Error::msg)?;
    apply_compaction_summary(history, start, end, &summary);
    Ok(true)
}
