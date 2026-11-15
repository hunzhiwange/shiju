//! 宿主具体类型组合与集成测试注册。
pub type AgentTool = tool_agents::agent_tool::AgentTool<super::agent_hosts::ToolAgentHost>;
pub use tool_agents::agent_tool::{AgentToolChildSessionCreated, AgentToolChildSessionCreatedCallback};
#[cfg(test)]
use tool_agents::{agent_tool::AgentToolAction, ports::ChildCoordinator};
#[cfg(test)]
use super::traits::{Tool, ToolResult, ToolCallResult};
#[cfg(test)]
use config_agent::DelegateAgentConfig;
#[cfg(test)]
use std::{collections::HashMap, sync::Arc};
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/agent_tool/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/agent_tool/communication_tests.rs"]
mod communication_tests;
#[cfg(test)]
#[path = "../../session/child_agent/collaboration_tests.rs"]
mod collaboration_tests;

#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use serde_json::{Value, json};
#[cfg(test)]
use session_port::child_agent::WaitCondition;
#[cfg(test)]
use crate::session::child_agent::coordinator::ChildAgentCoordinator;

#[cfg(test)]
type ChildExecutionGuard = tool_agents::agent_tool::ChildExecutionGuard<super::agent_hosts::ChildCoordinatorHost>;
