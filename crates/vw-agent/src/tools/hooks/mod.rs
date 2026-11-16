//! Shared tool hook coordination with the concrete host hook registry.
#[cfg(test)]
use crate::hooks::{HookResult, HookRunner};
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
pub(crate) type PreToolHook = tool_context::hooks::PreToolHook<super::context::AgentContextHost>;
#[cfg(test)]
mod tests;
