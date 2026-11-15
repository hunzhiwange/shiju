//! Concrete host binding for the shared execution orchestration.
use super::parsing::ParsedToolCall;
use crate::tools::{ScheduledToolBatch, Tool};
#[cfg(test)]
use crate::tools;
#[cfg(test)]
use crate::{tools::{ToolUseContext, ToolResult}, observability::{Observer, ObserverEvent}};
#[cfg(test)]
use std::{sync::Arc, time::Duration};
#[cfg(test)]
use tokio_util::sync::CancellationToken;
#[cfg(test)]
use agent_loop::core::errors::ToolLoopCancelled;
pub use agent_loop::execution::ToolExecutionOutcome;
pub(crate) use agent_loop::execution::{execute_tools_parallel, execute_tools_sequential};
pub(crate) fn schedule_tool_batches(calls: &[ParsedToolCall], registry: &[Box<dyn Tool>]) -> Vec<ScheduledToolBatch> {
    let calls: Vec<_> = calls.iter().map(|call| agent_loop::dispatcher::ParsedToolCall {
        name: call.name.clone(), arguments: call.arguments.clone(), tool_call_id: call.tool_call_id.clone(),
    }).collect();
    agent_loop::execution::schedule_tool_batches(&calls, registry)
}
#[cfg(test)]
pub(crate) fn should_execute_tools_in_parallel(calls: &[ParsedToolCall], registry: &[Box<dyn Tool>]) -> bool {
    matches!(schedule_tool_batches(calls, registry).as_slice(), [ScheduledToolBatch { mode: tools::ScheduledToolBatchMode::Parallel, .. }])
}
#[cfg(test)]
#[path = "../../../../libs/agent-loop/src/execution_tests.rs"]
mod execution_tests;
