use crate::tools::ToolRuntimeContext;
pub(crate) fn maybe_mark_todo_in_progress(
    session: &mut super::Session,
    ctx: &ToolRuntimeContext,
    tool_state: &mut super::ToolSessionState,
) { session_engine::runner_todos::maybe_mark_todo_in_progress::<crate::session::processor::runner_host::AgentRunnerHost>(session, ctx, tool_state) }

#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/todos_tests.rs"]
mod todos_tests;
