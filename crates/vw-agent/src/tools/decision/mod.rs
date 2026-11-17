//! Shared tool permission orchestration.
#[cfg(any(test, target_arch = "wasm32"))]
pub use tool_context::decision::{PermissionDecision, finalize_tool_input};
#[cfg(any(unix, test))]
use super::context::ToolUseContext;
#[cfg(any(unix, test))]
use serde_json::Value;
#[cfg(test)]
use tool_context::decision::resolve_permission_decision;
#[cfg(test)]
async fn request_interactive_approval(name: &str, pending: PermissionDecision, context: &ToolUseContext) -> PermissionDecision { tool_context::decision::request_interactive_approval(name, pending, context).await }
#[cfg(unix)]
pub(super) async fn request_control_scope_approval(name: &str, input: Value, context: &ToolUseContext) -> bool { tool_context::decision::request_control_scope_approval(name, input, context).await }
#[cfg(test)]
mod tests;
