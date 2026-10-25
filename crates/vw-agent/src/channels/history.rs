//! 通道历史记录管理模块
//!
//! 本模块提供了用于管理通道会话历史记录的工具函数，包括：
//! - 会话键生成：为不同级别的上下文生成唯一标识符
//! - 历史记录操作：追加、清理、压缩和回滚会话轮次
//! - 轮次规范化：处理和合并连续的消息轮次
//!
//! # 模块架构
//!
//! 历史记录管理采用分层键设计，支持：
//! - 单条消息级别的记忆（`conversation_memory_key`）
//! - 会话级别的历史（`conversation_history_key`）
//! - 线程隔离的上下文（通过 `thread_ts` 支持）
//!
//! # 使用场景
//!
//! 主要用于通道运行时上下文（`ChannelRuntimeContext`）中的会话状态管理，
//! 确保多用户、多通道、多线程的消息能够正确隔离和追踪。

use super::*;

/// 消息级记忆键包含完整聊天身份和平台消息 ID。
pub(crate) fn conversation_memory_key(msg: &traits::ChannelMessage) -> String {
    serde_json::to_string(&(ChannelChatScope::from_message(msg), &msg.id))
        .expect("channel memory identity contains only serializable fields")
}

/// 聊天身份包含租户、实例、房间、主题和发送者，结构化编码保留字段边界。
/// 执行路径使用 `scoped_history_key` 再加入已解析的项目 ID。
pub(crate) fn conversation_history_key(msg: &traits::ChannelMessage) -> String {
    serde_json::to_string(&ChannelChatScope::from_message(msg))
        .expect("channel identity contains only serializable fields")
}

/// 中断与历史使用完整聊天身份，包含租户、实例、房间、主题和发送者。
pub(crate) fn interruption_scope_key(msg: &traits::ChannelMessage) -> String {
    conversation_history_key(msg)
}

/// 规范化缓存的通道轮次
///
/// 该函数处理和规范化对话轮次序列，确保用户和助手消息正确交替。
/// 对于不符合交替模式的消息（如中断导致的连续用户消息），会进行合并处理。
///
/// # 参数
///
/// - `turns`: 原始的聊天消息向量，可能包含不符合交替模式的消息
///
/// # 返回值
///
/// 返回规范化后的聊天消息向量，其中：
/// - 用户和助手消息正确交替
/// - 不符合模式的消息已合并到相邻的同类消息中
///
/// # 处理逻辑
///
/// 1. 期望状态机：跟踪当前期望的消息角色（用户或助手）
/// 2. 正常情况：用户 -> 助手 -> 用户 -> 助手 的交替模式
/// 3. 异常处理：
///    - 连续的用户消息（助手被中断未持久化）：合并到前一个用户消息
///    - 连续的助手消息：合并到前一个助手消息
///
/// # 示例
///
/// ```ignore
/// let turns = vec![
///     ChatMessage { role: "user".into(), content: "Hello".into() },
///     ChatMessage { role: "user".into(), content: "World".into() }, // 异常：连续用户消息
/// ];
/// let normalized = normalize_cached_channel_turns(turns);
/// // 结果：一个包含合并内容 "Hello\n\nWorld" 的用户消息
/// ```
pub(crate) fn normalize_cached_channel_turns(turns: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut normalized = Vec::with_capacity(turns.len());
    // 状态标志：true 表示期望用户消息，false 表示期望助手消息
    let mut expecting_user = true;

    for turn in turns {
        match (expecting_user, turn.role.as_str()) {
            // 正常情况：收到期望的用户消息
            (true, "user") => {
                normalized.push(turn);
                expecting_user = false; // 下次期望助手消息
            }
            // 正常情况：收到期望的助手消息
            (false, "assistant") => {
                normalized.push(turn);
                expecting_user = true; // 下次期望用户消息
            }
            // 中断的通道轮次可能产生连续的用户消息（助手尚未持久化）
            // 采用合并策略而非丢弃，以保留完整的用户输入上下文
            (false, "user") | (true, "assistant") => {
                if let Some(last_turn) = normalized.last_mut() {
                    if !turn.content.is_empty() {
                        if !last_turn.content.is_empty() {
                            // 用双换行符分隔合并的内容，保持可读性
                            last_turn.content.push_str("\n\n");
                        }
                        last_turn.content.push_str(&turn.content);
                    }
                }
            }
            // 忽略其他角色（如 system）的消息
            _ => {}
        }
    }

    normalized
}

/// 清除发送者的历史记录
///
/// 从通道运行时上下文中完全移除指定发送者的所有对话历史。
/// 通常用于用户主动清除历史或重置会话状态。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文引用，包含会话历史存储
/// - `sender_key`: 发送者会话键，通常通过 `scoped_history_key` 生成
///
/// # 线程安全
///
/// 该函数会锁定 `conversation_histories` 互斥锁进行操作。
/// 如果锁被污染（poisoned），会恢复并继续操作，确保不会因 panic 而中断。
///
/// # 示例
///
/// ```ignore
/// let sender_key = scoped_history_key(&ctx, &msg);
/// clear_sender_history(&ctx, &sender_key);
/// ```
pub(crate) fn clear_sender_history(ctx: &ChannelRuntimeContext, sender_key: &str) {
    ctx.conversation_histories.lock().unwrap_or_else(|e| e.into_inner()).remove(sender_key);
}

/// 压缩发送者的历史记录
///
/// 对指定发送者的对话历史进行压缩，保留最近的 N 条消息，
/// 并对过长的消息内容进行截断处理，以控制内存使用和上下文长度。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文引用，包含会话历史存储
/// - `sender_key`: 发送者会话键
///
/// # 返回值
///
/// 返回 `bool` 表示是否成功执行了压缩操作：
/// - `true`: 存在历史记录且成功压缩
/// - `false`: 历史记录不存在、为空或压缩后为空
///
/// # 压缩策略
///
/// 1. **数量保留**：保留最近的 `CHANNEL_HISTORY_COMPACT_KEEP_MESSAGES` 条消息
/// 2. **内容截断**：单条消息超过 `CHANNEL_HISTORY_COMPACT_CONTENT_CHARS` 字符时截断
/// 3. **规范化**：压缩后对轮次进行规范化处理（合并连续消息）
///
/// # 性能考虑
///
/// 压缩操作会复制部分数据，适用于历史记录较长时的定期清理。
/// 在高频对话场景中，建议根据触发条件（如消息数量或时间间隔）进行压缩。
///
/// # 示例
///
/// ```ignore
/// if turns.len() > MAX_CHANNEL_HISTORY {
///     compact_sender_history(&ctx, &sender_key);
/// }
/// ```
pub(crate) fn compact_sender_history(ctx: &ChannelRuntimeContext, sender_key: &str) -> bool {
    let mut histories = ctx.conversation_histories.lock().unwrap_or_else(|e| e.into_inner());

    let Some(turns) = histories.get_mut(sender_key) else {
        return false;
    };

    if turns.is_empty() {
        return false;
    }

    // 计算保留的起始索引，保留最近 N 条消息
    let keep_from = turns.len().saturating_sub(CHANNEL_HISTORY_COMPACT_KEEP_MESSAGES);
    // 提取保留的消息并进行规范化
    let mut compacted = normalize_cached_channel_turns(turns[keep_from..].to_vec());

    // 对过长的消息内容进行截断
    for turn in &mut compacted {
        if turn.content.chars().count() > CHANNEL_HISTORY_COMPACT_CONTENT_CHARS {
            turn.content =
                truncate_with_ellipsis(&turn.content, CHANNEL_HISTORY_COMPACT_CONTENT_CHARS);
        }
    }

    // 如果压缩后为空，清除历史并返回 false
    if compacted.is_empty() {
        turns.clear();
        return false;
    }

    // 用压缩后的轮次替换原有历史
    *turns = compacted;
    true
}

/// 追加发送者的对话轮次
///
/// 向指定发送者的历史记录中追加一条新的对话消息。
/// 如果历史记录超过最大限制，会自动移除最早的消息。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文引用，包含会话历史存储
/// - `sender_key`: 发送者会话键
/// - `turn`: 要追加的聊天消息（用户或助手消息）
///
/// # 容量管理
///
/// 当历史记录长度超过 `MAX_CHANNEL_HISTORY` 时，会循环移除最早的消息，
/// 确保历史记录不会无限增长。
///
/// # 线程安全
///
/// 该函数会锁定 `conversation_histories` 互斥锁进行操作。
///
/// # 示例
///
/// ```ignore
/// let turn = ChatMessage {
///     role: "user".into(),
///     content: "Hello".into(),
/// };
/// append_sender_turn(&ctx, &sender_key, turn);
/// ```
pub(crate) fn append_sender_turn(ctx: &ChannelRuntimeContext, sender_key: &str, turn: ChatMessage) {
    let mut histories = ctx.conversation_histories.lock().unwrap_or_else(|e| e.into_inner());
    // 如果键不存在，创建新的历史记录向量
    let turns = histories.entry(sender_key.to_string()).or_default();
    turns.push(turn);
    // 超过最大历史长度时，移除最早的消息（FIFO 队列行为）
    while turns.len() > MAX_CHANNEL_HISTORY {
        turns.remove(0);
    }
}

/// 回滚孤立的用户轮次
///
/// 当用户消息未能得到助手回复时（如请求被取消或失败），
/// 该函数用于从历史记录中移除该孤立的用户消息，保持历史的一致性。
///
/// # 参数
///
/// - `ctx`: 通道运行时上下文引用，包含会话历史存储
/// - `sender_key`: 发送者会话键
/// - `expected_content`: 预期的用户消息内容，用于验证要移除的消息
///
/// # 返回值
///
/// 返回 `bool` 表示是否成功执行了回滚操作：
/// - `true`: 最后一条是匹配的用户消息，已成功移除
/// - `false`: 历史不存在，或最后一条消息不匹配
///
/// # 一致性保证
///
/// 通过 `expected_content` 参数确保只移除预期的消息，防止误删除。
/// 如果移除后历史为空，会清理整个发送者的历史记录条目。
///
/// # 使用场景
///
/// - 用户取消正在处理的请求
/// - 助手生成失败需要恢复状态
/// - 测试或调试时需要撤销最后的用户输入
///
/// # 示例
///
/// ```ignore
/// let user_input = "What is the weather?";
/// // 用户取消请求后回滚
/// if rollback_orphan_user_turn(&ctx, &sender_key, user_input) {
///     println!("Successfully rolled back orphan user turn");
/// }
/// ```
pub(crate) fn rollback_orphan_user_turn(
    ctx: &ChannelRuntimeContext,
    sender_key: &str,
    expected_content: &str,
) -> bool {
    let mut histories = ctx.conversation_histories.lock().unwrap_or_else(|e| e.into_inner());
    let Some(turns) = histories.get_mut(sender_key) else {
        return false;
    };

    // 验证最后一条是否为匹配内容的用户消息
    let should_pop =
        turns.last().is_some_and(|turn| turn.role == "user" && turn.content == expected_content);
    if !should_pop {
        return false;
    }

    // 移除该轮次
    turns.pop();
    // 如果历史为空，移除整个发送者条目以释放内存
    if turns.is_empty() {
        histories.remove(sender_key);
    }
    true
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
