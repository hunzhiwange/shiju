//! 消息响应处理模块
//!
//! 本模块负责处理 LLM（大语言模型）执行完成后返回的各种结果类型，包括：
//! - 成功响应：处理并返回模型生成的文本内容
//! - 取消响应：处理因新消息到达而被取消的请求
//! - 错误响应：处理各类错误情况（上下文溢出、工具迭代限制等）
//! - 超时响应：处理模型响应超时的情况
//!
//! # 主要功能
//!
//! - 响应内容的安全检查（金丝雀令牌防护）
//! - 钩子系统的集成（消息发送前的拦截和修改）
//! - 响应内容的净化处理
//! - 会话历史的更新和管理
//! - 多种错误情况的优雅降级处理
//!
//! # 与其他模块的关系
//!
//! 该模块是通道管理器（channel manager）的核心组成部分，与以下模块紧密协作：
//! - `security::CanaryGuard` - 提供响应内容的安全检查
//! - `hooks` - 提供消息生命周期钩子
//! - `memory` - 管理会话历史记录

use super::*;
pub(crate) use channel_runtime::manager::message_response::*;
pub(crate) async fn handle_llm_result(
    ctx: &ChannelRuntimeContext,
    msg: traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
    route: &ChannelRouteSelection,
    llm_result: LlmExecutionResult,
    history_key: &str,
    history: &[ChatMessage],
    history_len_before_tools: usize,
    started_at: Instant,
    draft_message_id: Option<&str>,
    user_turn_content: &str,
    turn_canary_token: Option<&str>,
    reaction_done_emoji: &str,
    canary_guard: &crate::app::agent::security::CanaryGuard,
) { channel_runtime::manager::message_response::handle_llm_result(&super::runtime_host::HOST, ctx, msg, target_channel, route, llm_result, history_key, history, history_len_before_tools, started_at, draft_message_id, user_turn_content, turn_canary_token, reaction_done_emoji, canary_guard).await }
pub(crate) async fn mark_retryable_reply(
    snapshot: &ChannelMessageSnapshot,
    id: &str,
    msg: &traits::ChannelMessage,
    reply: String,
) { channel_runtime::manager::message_response::mark_retryable_reply(&super::runtime_host::HOST, snapshot, id, msg, reply).await }
pub(crate) async fn reconcile_channel_reply(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
    channel: Option<&Arc<dyn Channel>>,
) -> bool { channel_runtime::manager::message_response::reconcile_channel_reply(&super::runtime_host::HOST, snapshot, msg, channel).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_response_tests.rs"]
mod message_response_tests;
