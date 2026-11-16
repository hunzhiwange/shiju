//! Concrete host binding; shared history algorithms live in agent-loop.
use model_types::ChatMessage;
pub use agent_loop::history::{trim_history, apply_compaction_summary, build_compaction_transcript};
struct HistoryHost;
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl agent_loop::history::HistoryHost for HistoryHost {
    async fn restore(session: &str, source: &[serde_json::Value]) -> Result<Vec<serde_json::Value>, String> { crate::session::compaction_checkpoint::restore(session, source).await }
    async fn save(session: &str, source: &[serde_json::Value], summary: &str, retained: Option<serde_json::Value>) -> Result<(), String> { crate::session::compaction_checkpoint::save(session, source, summary, retained).await }
    async fn summarize(session: String, source: Vec<serde_json::Value>, model: String, reliability: crate::config::ReliabilityConfig, limit: crate::provider::provider::ModelLimit) -> Result<String, String> { crate::session::compaction_summary::summarize_async(session, source, model, reliability, limit).await }
    async fn emit(session: &str, thread: &str, model: &str, automatic: bool, post: bool) -> anyhow::Result<()> { crate::plugins::hook_compact::emit::<crate::tools::context::AgentContextHost>(session, thread, model, automatic, post).await }
}
pub(crate) async fn compact_history_with_internal_service(history: &mut Vec<ChatMessage>, session: &str, model: &str, reliability: &crate::config::ReliabilityConfig, limit: &crate::provider::provider::ModelLimit) -> anyhow::Result<bool> {
    agent_loop::history::compact_history_with_internal_service::<HistoryHost>(history, session, model, reliability, limit).await
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
