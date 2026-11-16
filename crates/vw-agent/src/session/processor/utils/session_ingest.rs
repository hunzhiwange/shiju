use super::super::types::StreamEvent;
use crate::session::session::Session;
use crate::tools::ToolRuntimeContext;
use std::collections::HashSet;
pub(crate) fn push_user_dedup(session: &mut Session, content: String) { session_engine::runner_ingest::push_user_dedup(session, content) }
pub(crate) fn ingest_assistant_answer(
    session: &mut Session,
    answer: &str,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    ran_tool: &mut bool,
    tool_state: &mut super::super::ToolSessionState,
) -> String { session_engine::runner_ingest::ingest_assistant_answer::<crate::session::processor::runner_host::AgentRunnerHost>(session, answer, ctx, allowed_tools, on_event, ran_tool, tool_state) }
pub(crate) fn ingest_user_query(
    session: &mut Session,
    query: &str,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut super::super::ToolSessionState,
) { session_engine::runner_ingest::ingest_user_query::<crate::session::processor::runner_host::AgentRunnerHost>(session, query, ctx, allowed_tools, on_event, tool_state) }

#[cfg(test)]
#[path = "../../../../../libs/session-engine/tests/host_suites/session_ingest_tests.rs"]
mod session_ingest_tests;
