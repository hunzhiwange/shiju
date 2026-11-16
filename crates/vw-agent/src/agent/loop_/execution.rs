//! Concrete host binding for the shared execution orchestration.
#[cfg(test)]
use super::parsing::ParsedToolCall;
#[cfg(test)]
use crate::tools::Tool;
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
pub(crate) use agent_loop::execution::schedule_parsed_tool_batches as schedule_tool_batches;
#[cfg(test)]
#[path = "../../../../libs/agent-loop/src/execution_tests.rs"]
mod execution_tests;
