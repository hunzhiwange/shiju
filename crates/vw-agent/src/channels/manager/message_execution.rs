//! 消息执行模块
//!
//! 本模块负责在通道管理器中执行消息处理流程，将接收到的用户消息
//! 通过会话处理器发送给 LLM 进行处理，并返回执行结果。
//!
//! # 主要功能
//!
//! - 解析或创建发送者会话标识
//! - 构建会话处理器请求
//! - 管理执行超时和取消机制
//! - 返回结构化的 LLM 执行结果

use super::*;
pub(crate) use channel_runtime::manager::message_execution::*;
pub(crate) async fn run_message_execution(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    snapshot: &ChannelMessageSnapshot,
    session_history: Vec<crate::session::ui_types::ChatMessage>,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    timeout_budget_secs: u64,
    cancellation_token: &CancellationToken,
) -> LlmExecutionResult { channel_runtime::manager::message_execution::run_message_execution(&super::runtime_host::HOST, ctx, msg, route, snapshot, session_history, delta_tx, timeout_budget_secs, cancellation_token).await }
pub(crate) async fn stop_channel_execution(
    execution: impl std::future::Future<Output = anyhow::Result<ChannelProcessorResponse>>,
    abort: &CancellationToken,
    snapshot: &ChannelMessageSnapshot,
    session: &str,
) -> anyhow::Result<()> { channel_runtime::manager::message_execution::stop_channel_execution(&super::runtime_host::HOST, execution, abort, snapshot, session).await }
#[cfg(test)]
fn reap_channel_shell_processes(snapshot: &ChannelMessageSnapshot, session: &str) -> Result<()> { channel_runtime::host::ChannelHost::reap_shell(&super::runtime_host::HOST, snapshot, session) }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_execution_tests.rs"]
mod message_execution_tests;
