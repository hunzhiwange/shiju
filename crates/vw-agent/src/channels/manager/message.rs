//! 通道运行实现与宿主能力注入。
use super::*;
pub(crate) use channel_runtime::manager::message::*;
pub(crate) async fn prepare_streaming_ui_state(
    target_channel: Option<&Arc<dyn Channel>>,
    msg: &traits::ChannelMessage,
    expose_internal_tool_details: bool,
    use_streaming: bool,
) -> StreamingUiState { channel_runtime::manager::message::prepare_streaming_ui_state(&super::runtime_host::HOST, target_channel, msg, expose_internal_tool_details, use_streaming).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/message_tests.rs"]
mod message_tests;
