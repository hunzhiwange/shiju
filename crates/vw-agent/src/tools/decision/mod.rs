//! Shared tool permission orchestration.
pub use tool_context::decision::{PermissionDecision, finalize_tool_input};
#[cfg(any(unix, test))]
use super::context::ToolUseContext;
#[cfg(any(unix, test))]
use serde_json::Value;
#[cfg(test)]
use super::{Tool, ToolSpec};
#[cfg(test)]
use super::toolset::ToolCallError;
#[cfg(test)]
use crate::app::agent::security::SecurityPolicy;
#[cfg(test)]
use tool_context::decision::{finalize_permission_decision, resolve_permission_decision, command_input, mark_input_as_approved, render_reason, permission_request_from_decision, classify_message};
#[cfg(test)]
use crate::tools::context::shell_policy::shell_security_decision;
#[cfg(test)]
fn blocked_path(input: &Value, security: &SecurityPolicy) -> Option<String> { tool_context::decision::blocked_path::<super::context::AgentContextHost>(input, security) }
#[cfg(test)]
fn security_decision(spec: &ToolSpec, input: Value, context: &ToolUseContext, approved: bool) -> PermissionDecision { tool_context::decision::security_decision(spec, input, context, approved) }
#[cfg(test)]
async fn request_interactive_approval(name: &str, pending: PermissionDecision, context: &ToolUseContext) -> PermissionDecision { tool_context::decision::request_interactive_approval(name, pending, context).await }
#[cfg(unix)]
pub(super) async fn request_control_scope_approval(name: &str, input: Value, context: &ToolUseContext) -> bool { tool_context::decision::request_control_scope_approval(name, input, context).await }
#[cfg(test)]
mod tests;
