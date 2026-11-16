use super::types::StreamEvent;
use crate::session::ui_types as models;
use std::collections::HashSet;
pub(crate) fn maybe_update_todos_after_work(
    session: &mut super::Session,
    ctx: &crate::app::agent::tools::ToolRuntimeContext,
    model: Option<String>,
    base_system: &str,
    allowed_tools: &HashSet<String>,
    llm_messages: &mut Vec<serde_json::Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
    reliability: &crate::app::agent::config::ReliabilityConfig,
) -> bool { session_engine::runner_todo_updates::maybe_update_todos_after_work::<crate::session::processor::runner_host::AgentRunnerHost>(session, ctx, model, base_system, allowed_tools, llm_messages, total_usage, on_event, tool_state, reliability) }
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/todo_updates_tests.rs"]
mod todo_updates_tests;
