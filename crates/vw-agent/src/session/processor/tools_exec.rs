//! Concrete tool execution bindings; engine owns execution policy and recording.
use crate::tools as tools;
#[cfg(test)]
use super::ToolSessionState;
#[cfg(test)]
use session_engine::tool_output::compact_tool_output;
pub(crate) use session_engine::tools_exec::*;
pub(crate) type AgentToolPlannedCall = session_engine::tools_exec::AgentToolPlannedCall<crate::tools::context::AgentContextHost>;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use crate::session::child_agent::coordinator::{ChildAgentCoordinator, ChildAgentTaskSnapshot, ChildAgentTaskState};
#[cfg(test)]
use super::types::{StreamEvent, ToolCallStatus, ToolEventKind};
/// Wait at the final-answer boundary, then deliver each unconsumed result to the model.
pub(crate) fn integrate_agent_tool_children(context: &tools::ToolUseContext, messages: &mut Vec<serde_json::Value>, tool_state: &super::ToolSessionState, cancellation: Option<&tokio_util::sync::CancellationToken>, budget_exhausted: bool) -> anyhow::Result<bool> {
 session_engine::runner::integrate_agent_tool_children::<super::runner_host::AgentRunnerHost>(context, messages, tool_state, cancellation, budget_exhausted)
}

#[cfg(test)]
fn agent_tool_snapshot_status(snapshot: &ChildAgentTaskSnapshot, waiting: bool) -> &'static str {
    if waiting && !snapshot.state.is_terminal() {
        return "waiting";
    }
    match snapshot.state {
        ChildAgentTaskState::Queued | ChildAgentTaskState::Running => "running",
        ChildAgentTaskState::Succeeded => "completed",
        ChildAgentTaskState::Failed => "failed",
        ChildAgentTaskState::Cancelled => "cancelled",
    }
}

#[cfg(test)]
fn agent_tool_snapshot_payload(
    snapshot: &ChildAgentTaskSnapshot,
    waiting: bool,
) -> serde_json::Value {
    let output = serde_json::to_string(&serde_json::json!({
        "action": "status",
        "state": snapshot.state,
        "parent_session_id": snapshot.parent_session_id,
        "child_session_id": snapshot.child_session_id,
        "agent": snapshot.agent,
        "task": snapshot.task,
        "output": snapshot.output,
        "error": snapshot.error,
    }))
    .expect("child agent snapshot should serialize");
    serde_json::json!({
        "status": agent_tool_snapshot_status(snapshot, waiting),
        "title": "AgentTool",
        "metadata": {
            "parent_session_id": snapshot.parent_session_id,
            "child_session_id": snapshot.child_session_id,
            "agent": snapshot.agent,
            "task": snapshot.task,
        },
        "output": output,
    })
}

#[cfg(test)]
fn finalize_agent_tool_children_with(
    coordinator: &ChildAgentCoordinator,
    parent_session_id: &str,
    session: &mut super::Session,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    child_session_ids: &[String],
) -> bool {
    if child_session_ids.is_empty() {
        return true;
    }

    let mut waiting_emitted = std::collections::HashSet::new();
    loop {
        let outcome = match coordinator.wait_for_descendants_blocking(
            parent_session_id,
            child_session_ids,
            Duration::from_millis(250),
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = coordinator.cancel_descendants(parent_session_id, child_session_ids);
                on_event(StreamEvent::Error(format!("AgentTool 子会话终态同步失败：{error}")));
                return false;
            }
        };

        if outcome.timed_out {
            for snapshot in outcome.tasks.iter().filter(|snapshot| {
                snapshot.parent_session_id == parent_session_id
                    && !snapshot.state.is_terminal()
                    && waiting_emitted.insert(snapshot.child_session_id.clone())
            }) {
                let payload = agent_tool_snapshot_payload(snapshot, true);
                if !on_event(tool_update_event(
                    snapshot.child_session_id.clone(),
                    "AgentTool",
                    payload,
                )) {
                    return false;
                }
            }
            continue;
        }

        for snapshot in
            outcome.tasks.iter().filter(|snapshot| snapshot.parent_session_id == parent_session_id)
        {
            let payload = agent_tool_snapshot_payload(snapshot, false);
            if !on_event(tool_update_event(
                snapshot.child_session_id.clone(),
                "AgentTool",
                payload.clone(),
            )) {
                return false;
            }
            let message = format!("tool AgentTool\n{payload}\n");
            session.push(super::Role::Tool, message);
        }
        return true;
    }
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
