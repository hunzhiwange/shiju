//! Typed tool execution adapter for planning ingestion.
use super::types::StreamEvent;
use crate::tools::ToolRuntimeContext;
use std::collections::HashSet;
#[cfg(test)]
use session_engine::planning::build_todowrite_from_text;
struct AgentPlanningHost;
impl session_engine::planning::PlanningHost for AgentPlanningHost {
    type Context = ToolRuntimeContext;
    type State = super::ToolSessionState;
    fn run_tool_and_record(session: &mut super::Session, name: &str, input: &str, ctx: &Self::Context, emit: bool, on_event: &mut dyn FnMut(StreamEvent) -> bool, state: &mut Self::State) {
        let mut forward = |event| on_event(event);
        let _ = super::tools_exec::run_tool_and_record(session, name, input, ctx, emit, &mut forward, state);
    }
}
pub(crate) fn ingest_planning_answer_todowrite_only(
    session: &mut super::Session,
    answer: &str,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    on_event: &mut dyn FnMut(StreamEvent) -> bool,
    tool_state: &mut super::ToolSessionState,
) -> bool { session_engine::planning::ingest_planning_answer_todowrite_only::<AgentPlanningHost>(session, answer, ctx, allowed_tools, on_event, tool_state) }
#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/planning_tests.rs"]
mod planning_tests;
