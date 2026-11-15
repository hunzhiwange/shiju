//! 通道消息调度；控制命令不等待模型并发许可。
use super::*;
pub(crate) use channel_runtime::manager::dispatch::*;
pub(crate) async fn run_message_dispatch_loop(
    rx: tokio::sync::mpsc::Receiver<traits::ChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
) { channel_runtime::manager::dispatch::run_message_dispatch_loop(&super::runtime_host::HOST, rx, ctx, max_in_flight_messages).await }
