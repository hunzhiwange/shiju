//! Trusted parent execution carries child identity across worker threads.
use crate::tools::ToolUseContext;
use anyhow::Result;
use std::sync::Arc;

#[cfg(test)]
pub(crate) async fn emit(
    context: Option<Arc<ToolUseContext>>,
    parent: &str,
    child: &str,
    agent: &str,
    output: Option<&str>,
    stop: bool,
) -> Result<Vec<String>> {
    emit_boundary(context, parent, child, agent, output, stop.then_some(false)).await
}

pub(crate) async fn emit_boundary(
    context: Option<Arc<ToolUseContext>>,
    parent: &str,
    child: &str,
    agent: &str,
    output: Option<&str>,
    stop: Option<bool>,
) -> Result<Vec<String>> {
    plugin_hooks::hook_subagent::emit_boundary::<crate::tools::context::AgentContextHost>(context, parent, child, agent, output, stop).await
}

pub use plugin_hooks::hook_subagent::{validate, contexts, parse};
