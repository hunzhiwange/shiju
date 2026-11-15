//! 原进度协议常量绑定；输出规则由agent-loop持有。
use anyhow::Result;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use super::super::super::progress::{DRAFT_CLEAR_SENTINEL, DRAFT_PROGRESS_SENTINEL};
#[cfg(test)]
use super::super::errors::ToolLoopCancelled;
#[cfg(test)]
#[path = "output_tests.rs"]
mod output_tests;
fn wire() -> agent_loop::core::output::ProgressWire {
    agent_loop::core::output::ProgressWire { clear: DRAFT_CLEAR_SENTINEL, progress: DRAFT_PROGRESS_SENTINEL }
}
pub(super) async fn update_thinking_progress(tx: Option<&Sender<String>>, iteration: usize) {
    agent_loop::core::output::update_thinking_progress(&wire(), tx, iteration).await
}
pub(super) async fn update_tool_call_progress(tx: Option<&Sender<String>>, count: usize, secs: u64) {
    agent_loop::core::output::update_tool_call_progress(&wire(), tx, count, secs).await
}
pub(super) async fn send_retry_progress(tx: Option<&Sender<String>>) {
    agent_loop::core::output::send_retry_progress(&wire(), tx).await
}
pub(super) async fn stream_final_response(text: &str, tx: Option<&Sender<String>>, token: Option<&CancellationToken>) -> Result<()> {
    agent_loop::core::output::stream_final_response(&wire(), text, tx, token).await
}
