use super::types::StreamEvent;
use crate::tools::{ToolRuntimeContext, todo};
pub(crate) fn has_incomplete_todos(ctx: &ToolRuntimeContext) -> bool { session_engine::runner_todos::has_incomplete_todos::<crate::session::processor::runner_host::AgentRunnerHost>(ctx) }
pub(crate) fn read_todos_or_empty(ctx: &ToolRuntimeContext) -> Vec<todo::Todo> { session_engine::runner_todos::read_todos_or_empty::<crate::session::processor::runner_host::AgentRunnerHost>(ctx) }
pub(crate) fn build_todo_status_patches(
    items: &[todo::Todo],
    target_status: &str,
    only_first_incomplete: bool,
) -> Vec<serde_json::Value> { session_engine::runner_todos::build_todo_status_patches(items, target_status, only_first_incomplete) }
pub(crate) fn maybe_mark_todo_in_progress(
    session: &mut super::Session,
    ctx: &ToolRuntimeContext,
    tool_state: &mut super::ToolSessionState,
) { session_engine::runner_todos::maybe_mark_todo_in_progress::<crate::session::processor::runner_host::AgentRunnerHost>(session, ctx, tool_state) }

#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/todos_tests.rs"]
mod todos_tests;
