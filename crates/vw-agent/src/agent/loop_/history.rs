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
//! - 在总结失败时回退到确定性截断策略

use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::util::truncate_with_ellipsis;
use anyhow::Result;

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
use std::fmt::Write;

/// 压缩后保留的最近非系统消息数量
///
/// 当执行历史压缩时，会保留这么多条最近的消息不参与压缩，
/// 确保最新的对话上下文保持完整。
const COMPACTION_KEEP_RECENT_MESSAGES: usize = 20;

/// 传递给总结器的源文本最大字符数
///
/// 为了控制 API 调用成本和响应时间，传递给 LLM 进行总结的
/// 对话文本将被限制为此字符数。超出部分将被截断。
const COMPACTION_MAX_SOURCE_CHARS: usize = 12_000;

/// 存储的压缩摘要最大字符数
///
/// 压缩后的摘要将被限制为此字符数，以防止压缩后的摘要
/// 本身占用过多空间，违背压缩的初衷。
const COMPACTION_MAX_SUMMARY_CHARS: usize = 2_000;

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
/// 返回格式化后的对话文本字符串。如果文本长度超过
/// `COMPACTION_MAX_SOURCE_CHARS`，将被截断并添加省略号。
///
/// # 格式示例
///
/// ```text
/// USER: 你好，请帮我分析这段代码
/// ASSISTANT: 好的，我来帮你分析...
/// USER: 那性能优化怎么做？
/// ASSISTANT: 关于性能优化，我建议...
/// ```
pub(crate) fn build_compaction_transcript(messages: &[ChatMessage]) -> String {
    let mut transcript = String::new();

    // 将每条消息转换为 "角色: 内容" 格式
    for msg in messages {
        let role = msg.role.to_uppercase();
        let _ = writeln!(transcript, "{role}: {}", msg.content.trim());
    }

    // 如果文本过长，截断以控制 API 调用成本
    if transcript.chars().count() > COMPACTION_MAX_SOURCE_CHARS {
        truncate_with_ellipsis(&transcript, COMPACTION_MAX_SOURCE_CHARS)
    } else {
        transcript
    }
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

/// 自动压缩对话历史
///
/// 当对话历史超过指定限制时，自动使用 LLM 将旧消息压缩为摘要。
/// 此函数会保留系统提示和最近的消息，只压缩中间的旧消息。
///
/// # 参数
///
/// - `history`: 待压缩的对话历史向量，可能被原地修改
/// - `provider`: 用于调用 LLM 进行总结的 Provider 实例
/// - `model`: 用于总结的模型名称
/// - `max_history`: 触发压缩的最大非系统消息数量阈值
///
/// # 返回值
///
/// 返回 `Result<bool>`：
/// - `Ok(true)`: 执行了压缩操作
/// - `Ok(false)`: 历史未超过限制，未执行压缩
/// - `Err(...)`: 压缩过程中发生错误
///
/// # 压缩流程
///
/// 1. 检查非系统消息数量是否超过阈值
/// 2. 确定要压缩的消息范围（保留最近 N 条）
/// 3. 构建待压缩消息的文本副本
/// 4. 调用 LLM 生成摘要，保留关键信息
/// 5. 用摘要消息替换旧消息
///
/// # 摘要内容
///
/// LLM 被指示保留以下信息：
/// - 用户偏好
/// - 承诺和约定
/// - 决策
/// - 未解决的任务
/// - 关键事实
///
/// 并省略：
/// - 无实质内容的对话
/// - 重复的寒暄
/// - 冗长的工具日志
///
/// # 错误处理
///
/// 如果摘要调用失败，返回错误并保留完整历史；不以本地截断伪造摘要。
///
/// # 示例
///
/// ```ignore
/// let mut history = vec![/* 大量消息 */];
/// let provider = OpenAIProvider::new(api_key);
///
/// // 当历史超过 50 条时自动压缩
/// let compressed = compact_history_with_internal_service(&mut history, &provider, "gpt-4", 50).await?;
///
/// if compressed {
///     println!("历史已压缩");
/// }
/// ```
pub(crate) async fn compact_history_with_internal_service(
    history: &mut Vec<ChatMessage>,
    provider: &dyn Provider,
    model: &str,
    max_history: usize,
) -> Result<bool> {
    // 检查是否存在系统提示消息
    let has_system = history.first().map_or(false, |m| m.role == "system");

    // 计算非系统消息数量
    let non_system_count = if has_system { history.len().saturating_sub(1) } else { history.len() };

    // 如果未超过阈值，无需压缩
    if non_system_count <= max_history {
        return Ok(false);
    }

    // 确定压缩范围的起始位置
    let start = if has_system { 1 } else { 0 };

    // 计算保留的最近消息数量（不超过实际非系统消息数）
    let keep_recent = COMPACTION_KEEP_RECENT_MESSAGES.min(non_system_count);

    // 计算需要压缩的消息数量
    let compact_count = non_system_count.saturating_sub(keep_recent);

    // 如果没有消息需要压缩，直接返回
    if compact_count == 0 {
        return Ok(false);
    }

    // 确定压缩范围的结束位置
    let compact_end = start + compact_count;

    // 提取待压缩的消息
    let to_compact: Vec<ChatMessage> = history[start..compact_end].to_vec();

    // 构建压缩源文本
    let transcript = build_compaction_transcript(&to_compact);

    // QueryEngine 没有持久化消息 ID；复用内部服务的无工具提示契约，而不是
    // 使用另一种摘要语义或本地回退策略。
    let summarizer_user = crate::session::compaction::internal_summary_prompt(&transcript);
    let summary_raw = provider
        .chat_with_system(None, &summarizer_user, model, 0.2)
        .await
        .map_err(|error| anyhow::anyhow!("internal compaction model request failed: {error}"))?;

    // 确保摘要不超过最大长度限制
    let summary = truncate_with_ellipsis(&summary_raw, COMPACTION_MAX_SUMMARY_CHARS);
    if summary.trim().is_empty() {
        anyhow::bail!("internal compaction generated an empty summary");
    }

    // 应用压缩摘要到历史记录
    apply_compaction_summary(history, start, compact_end, &summary);

    Ok(true)
}
