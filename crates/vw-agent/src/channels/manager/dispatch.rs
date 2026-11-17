//! 通道运行实现与宿主能力注入。
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::manager::dispatch::*;
pub(crate) async fn run_message_dispatch_loop(
    rx: tokio::sync::mpsc::Receiver<traits::ChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
) { channel_runtime::manager::dispatch::run_message_dispatch_loop(&super::runtime_host::HOST, rx, ctx, max_in_flight_messages).await }
