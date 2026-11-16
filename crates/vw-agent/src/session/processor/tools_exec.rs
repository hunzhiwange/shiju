//! Concrete tool execution bindings; engine owns execution policy and recording.
use crate::tools as tools;
#[cfg(test)]
use super::ToolSessionState;
pub(crate) use session_engine::tools_exec::*;
pub(crate) type AgentToolPlannedCall = session_engine::tools_exec::AgentToolPlannedCall<crate::tools::context::AgentContextHost>;
#[cfg(test)]
use super::types::{StreamEvent, ToolCallStatus, ToolEventKind};
/// Wait at the final-answer boundary, then deliver each unconsumed result to the model.
pub(crate) fn integrate_agent_tool_children(context: &tools::ToolUseContext, messages: &mut Vec<serde_json::Value>, tool_state: &super::ToolSessionState, cancellation: Option<&tokio_util::sync::CancellationToken>, budget_exhausted: bool) -> anyhow::Result<bool> {
 session_engine::runner::integrate_agent_tool_children::<super::runner_host::AgentRunnerHost>(context, messages, tool_state, cancellation, budget_exhausted)
}





#[cfg(test)]
#[path = "../../../../libs/session-engine/tests/host_suites/tools_exec_tests.rs"]
mod tools_exec_tests;

impl session_engine::tools_exec::ToolExecutionHost for crate::tools::context::AgentContextHost {
    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_native_search(discovery: &Self::ToolDiscoverySession, call: &model_types::ToolCall) -> anyhow::Result<()> { discovery.prepare_native_search(call) }
    #[cfg(not(target_arch = "wasm32"))]
    fn take_native_output(discovery: &Self::ToolDiscoverySession, call: &model_types::ToolCall) -> anyhow::Result<aisdk::core::Message> { discovery.take_native_output(call) }
}
