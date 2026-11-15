//! Synchronous command control effects; diagnostics never expose raw command output.
use anyhow::Result;

pub(crate) fn bind_input(
    payload: &mut serde_json::Value,
    event: crate::plugins::PluginHookEvent,
) -> Result<()> {
    plugin_hooks::hook_command::bind_input::<crate::tools::context::AgentContextHost>(payload, event)
}

/// Keep hook context in the owning tool turn; never share it through handler state.
pub(crate) fn retain_context(
    text: &str,
    limit: Option<usize>,
    data: &std::path::Path,
) -> Result<()> {
    plugin_hooks::hook_command::retain_context::<crate::tools::context::AgentContextHost>(text, limit, data)
}

pub use plugin_hooks::hook_command::{interpret, prepare_data_directory};
