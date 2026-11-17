//! 通道运行实现与宿主能力注入。
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
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
