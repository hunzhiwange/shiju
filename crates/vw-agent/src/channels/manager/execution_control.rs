//! 通道运行实现与宿主能力注入。
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::manager::execution_control::*;
pub(crate) async fn process_channel_message(
    ctx: Arc<ChannelRuntimeContext>,
    msg: traits::ChannelMessage,
    cancellation: CancellationToken,
) { channel_runtime::manager::execution_control::process_channel_message(&super::runtime_host::HOST, ctx, msg, cancellation).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/execution_control_tests.rs"]
mod tests;
