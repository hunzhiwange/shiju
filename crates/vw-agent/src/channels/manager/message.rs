//! 通道入站消息的主处理流程。
//!
//! 本模块负责把一条外部通道消息串联到代理执行：应用入站钩子、处理运行时
//! 命令、加载会话历史、构造系统提示、启动草稿/输入状态反馈，并最终把 LLM
//! 结果交给响应层。安全相关的语义防护和 canary token 也在这里按消息轮次接入。

use super::*;
pub(crate) use channel_runtime::manager::message::*;
pub(crate) async fn process_channel_message_inner(
    ctx: Arc<ChannelRuntimeContext>,
    msg: traits::ChannelMessage,
    cancellation_token: CancellationToken,
) { channel_runtime::manager::message::process_channel_message_inner(&super::runtime_host::HOST, ctx, msg, cancellation_token).await }
pub(crate) async fn prepare_streaming_ui_state(
    target_channel: Option<&Arc<dyn Channel>>,
    msg: &traits::ChannelMessage,
    expose_internal_tool_details: bool,
    use_streaming: bool,
) -> StreamingUiState { channel_runtime::manager::message::prepare_streaming_ui_state(&super::runtime_host::HOST, target_channel, msg, expose_internal_tool_details, use_streaming).await }
pub(crate) fn spawn_draft_updater(
    delta_rx: Option<tokio::sync::mpsc::Receiver<String>>,
    draft_message_id: Option<&str>,
    target_channel: Option<&Arc<dyn Channel>>,
    reply_target: String,
    expose_internal_tool_details: bool,
) -> Option<tokio::task::JoinHandle<()>> { channel_runtime::manager::message::spawn_draft_updater(&super::runtime_host::HOST, delta_rx, draft_message_id, target_channel, reply_target, expose_internal_tool_details) }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_tests.rs"]
mod message_tests;
