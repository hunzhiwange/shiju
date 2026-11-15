//! Current-input dispatch; restored history is never replayed through hooks.
use crate::tools::ToolUseContext;
use anyhow::Result;
use std::sync::Arc;

pub(crate) async fn submit(context: Arc<ToolUseContext>, prompt: &str) -> Result<()> {
    plugin_hooks::hook_prompt::submit::<crate::tools::context::AgentContextHost>(context, prompt).await
}

pub use plugin_hooks::hook_prompt::{validate, parse};
