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

pub(crate) fn clear_sender_history(ctx: &ChannelRuntimeContext, sender_key: &str) {
    channel_support::history::clear_sender_history(&ctx.conversation_histories, sender_key)
}

pub(crate) fn append_sender_turn(ctx: &ChannelRuntimeContext, sender_key: &str, turn: ChatMessage) {
    channel_support::history::append_sender_turn(&ctx.conversation_histories, sender_key, turn)
}

pub(crate) fn rollback_orphan_user_turn(ctx: &ChannelRuntimeContext, sender_key: &str, expected_content: &str) -> bool {
    channel_support::history::rollback_orphan_user_turn(&ctx.conversation_histories, sender_key, expected_content)
}


#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
