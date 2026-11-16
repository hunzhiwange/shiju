//! Completion control is scoped to a host-owned turn.
use crate::tools::ToolUseContext;
use anyhow::Result;
use std::sync::Arc;

pub use plugin_hooks::hook_stop::Outcome;

pub(crate) async fn emit(
    context: Arc<ToolUseContext>,
    interrupt: bool,
    active: bool,
    message: Option<&str>,
) -> Result<Outcome> {
    plugin_hooks::hook_stop::emit::<crate::tools::context::AgentContextHost>(context, interrupt, active, message).await
}
