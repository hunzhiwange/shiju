//! 当前聊天执行的取消与代次；直到处理器和反馈退出才释放忙碌状态。
use super::*;
pub(crate) use channel_runtime::manager::execution_control::*;
pub(crate) async fn process_channel_message(
    ctx: Arc<ChannelRuntimeContext>,
    msg: traits::ChannelMessage,
    cancellation: CancellationToken,
) { channel_runtime::manager::execution_control::process_channel_message(&super::runtime_host::HOST, ctx, msg, cancellation).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/execution_control_tests.rs"]
mod tests;
