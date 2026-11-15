//! Compact hooks receive metadata only; process failures do not block compaction.
use anyhow::Result;

pub(crate) async fn emit(
    session: &str,
    turn: &str,
    model: &str,
    auto: bool,
    post: bool,
) -> Result<()> {
    plugin_hooks::hook_compact::emit::<crate::tools::context::AgentContextHost>(session, turn, model, auto, post).await
}

/// Manual requests use the same trusted execution factory as normal turns.
pub(crate) async fn context(
    session: &str,
    turn: &str,
    model: &str,
) -> Result<std::sync::Arc<crate::tools::ToolUseContext>> {
    plugin_hooks::hook_compact::context::<crate::tools::context::AgentContextHost>(session, turn, model).await
}

pub use plugin_hooks::hook_compact::{validate, parse};
