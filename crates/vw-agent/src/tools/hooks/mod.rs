//! Shared tool hook coordination with the concrete host hook registry.
#[cfg(test)]
use super::decision::PermissionDecision;
#[cfg(test)]
use super::{ToolCallResult, ToolResult};
#[cfg(test)]
use crate::hooks::{HookResult, HookRunner};
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
pub(crate) type PreToolHook = tool_context::hooks::PreToolHook<super::context::AgentContextHost>;
#[cfg(test)]
pub(crate) type PostToolHook = tool_context::hooks::PostToolHook<super::context::AgentContextHost>;
#[cfg(test)]
use tool_context::hooks::PermissionHook;
#[cfg(test)]
mod tests;
