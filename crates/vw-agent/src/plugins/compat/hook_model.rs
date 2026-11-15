//! Observation-only events carry host metadata, never provider options or messages.
use serde_json::Value;

pub(crate) async fn emit(status: Option<&str>, usage: Value) {
    plugin_hooks::hook_model::emit::<crate::tools::context::AgentContextHost>(status, usage).await
}

pub use plugin_hooks::hook_model::{validate, token_usage};
