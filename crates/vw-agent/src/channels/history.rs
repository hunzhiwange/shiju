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
    channel_support::history::conversation_memory_key(msg, crate::session::ui_store::current_tenant_scope())
}

/// 聊天身份包含租户、实例、房间、主题和发送者，结构化编码保留字段边界。
/// 执行路径使用 `scoped_history_key` 再加入已解析的项目 ID。
pub(crate) fn conversation_history_key(msg: &traits::ChannelMessage) -> String {
    channel_support::history::conversation_history_key(msg, crate::session::ui_store::current_tenant_scope())
}

/// 中断与历史使用完整聊天身份，包含租户、实例、房间、主题和发送者。
pub(crate) fn interruption_scope_key(msg: &traits::ChannelMessage) -> String {
    conversation_history_key(msg)
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
    channel_support::history::clear_sender_history(&ctx.conversation_histories, sender_key)
}

/// Append raw history without changing earlier message boundaries or contents.
/// Model context is bounded by the session processor's checkpoints.
pub(crate) fn append_sender_turn(ctx: &ChannelRuntimeContext, sender_key: &str, turn: ChatMessage) {
    channel_support::history::append_sender_turn(&ctx.conversation_histories, sender_key, turn)
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
pub(crate) fn rollback_orphan_user_turn(ctx: &ChannelRuntimeContext, sender_key: &str, expected_content: &str) -> bool {
    channel_support::history::rollback_orphan_user_turn(&ctx.conversation_histories, sender_key, expected_content)
}


#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
