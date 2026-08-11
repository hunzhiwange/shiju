//! Desktop Chat 单回合执行预算。

pub(crate) const DEFAULT_MAX_TOOL_ITERATIONS: usize = 50_000;
pub(crate) const DEFAULT_MAX_HISTORY_MESSAGES: usize = 100;
pub(crate) const MAX_HISTORY_MESSAGES: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChatTurnPolicy {
    pub(crate) max_tool_iterations: usize,
    pub(crate) max_history_messages: usize,
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
        let max_history_messages =
            config.agent.max_history_messages.max(1).min(MAX_HISTORY_MESSAGES);
        let max_history_messages = if config.agent.max_history_messages == 0 {
            DEFAULT_MAX_HISTORY_MESSAGES
        } else {
            max_history_messages
        };

        Self { max_tool_iterations, max_history_messages }
    }
}

#[cfg(test)]
#[path = "turn_policy_tests.rs"]
mod turn_policy_tests;
