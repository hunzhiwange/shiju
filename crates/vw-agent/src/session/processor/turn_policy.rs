//! Desktop Chat 单回合执行预算。

pub(crate) const DEFAULT_MAX_TOOL_ITERATIONS: usize = 50_000;
pub(crate) const DEFAULT_MAX_HISTORY_MESSAGES: usize = 100;
pub(crate) const MAX_HISTORY_MESSAGES: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChatTurnPolicy {
    pub(crate) max_tool_iterations: usize,
    pub(crate) max_history_messages: usize,
    pub(crate) parallel_tools: bool,
}

impl ChatTurnPolicy {
    pub(crate) fn from_config(config: &crate::config::Config, agent_key: Option<&str>) -> Self {
        let selected_agent =
            agent_key.map(str::trim).filter(|key| !key.is_empty()).unwrap_or("main");
        let configured_max_tool_iterations = if selected_agent == "main" {
            config.agent.max_tool_iterations
        } else {
            config
                .agents
                .get(selected_agent)
                .map(|agent| agent.max_iterations)
                .unwrap_or(config.agent.max_tool_iterations)
        };
        let max_tool_iterations = (configured_max_tool_iterations > 0)
            .then_some(configured_max_tool_iterations)
            .unwrap_or(DEFAULT_MAX_TOOL_ITERATIONS);
        let configured_max_history_messages = if selected_agent == "main" {
            config.agent.max_history_messages
        } else {
            config
                .agents
                .get(selected_agent)
                .map(|agent| agent.max_history_messages)
                .unwrap_or(config.agent.max_history_messages)
        };
        let max_history_messages =
            configured_max_history_messages.max(1).min(MAX_HISTORY_MESSAGES);
        let max_history_messages = if configured_max_history_messages == 0 {
            DEFAULT_MAX_HISTORY_MESSAGES
        } else {
            max_history_messages
        };

        let parallel_tools = if selected_agent == "main" {
            config.agent.parallel_tools
        } else {
            config
                .agents
                .get(selected_agent)
                .map(|agent| agent.parallel_tools)
                .unwrap_or(config.agent.parallel_tools)
        };

        Self { max_tool_iterations, max_history_messages, parallel_tools }
    }
}

#[cfg(test)]
#[path = "turn_policy_tests.rs"]
mod turn_policy_tests;
