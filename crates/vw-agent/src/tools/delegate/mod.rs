//! 宿主具体类型组合与集成测试注册。
pub type DelegateTool = tool_agents::delegate::DelegateTool<super::agent_hosts::ToolAgentHost>;
#[cfg(test)]
use tool_agents::delegate::DELEGATE_TIMEOUT_SECS;
#[cfg(test)]
use config_agent::DelegateAgentConfig;
#[cfg(test)]
use std::{collections::HashMap, sync::Arc};
#[cfg(test)]
use super::traits::{Tool, ToolResult};
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/delegate/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/delegate/host_execution_tests.rs"]
mod execution;
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/delegate/host_agentic_tests.rs"]
mod agentic;
