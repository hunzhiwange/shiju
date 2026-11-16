//! Desktop Chat 单回合执行预算。

use config_agent::DEFAULT_MAX_ITERATIONS;
pub const DEFAULT_MAX_HISTORY_MESSAGES: usize = 100;
pub const MAX_HISTORY_MESSAGES: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatTurnPolicy {
    pub max_iterations: usize,
    pub max_history_messages: usize,
    pub parallel_tools: bool,
}

impl ChatTurnPolicy {
    pub fn from_config(
        config: &agent_config::runtime_schema::Config,
        agent_key: Option<&str>,
    ) -> Self {
        let selected_agent =
            agent_key.map(str::trim).filter(|key| !key.is_empty()).unwrap_or("main");
        let agents = config_agent::merged_agent_configs(&config.agents);
        let main_agent = agents.get("main").expect("builtin main agent configuration exists");
        let selected_agent = agents.get(selected_agent).unwrap_or(main_agent);
        let configured_max_iterations = selected_agent.max_iterations;
        let max_iterations = (configured_max_iterations > 0)
            .then_some(configured_max_iterations)
            .unwrap_or(DEFAULT_MAX_ITERATIONS);
        let configured_max_history_messages = selected_agent.max_history_messages;
        let max_history_messages = configured_max_history_messages.max(1).min(MAX_HISTORY_MESSAGES);
        let max_history_messages = if configured_max_history_messages == 0 {
            DEFAULT_MAX_HISTORY_MESSAGES
        } else {
            max_history_messages
        };

        let parallel_tools = selected_agent.parallel_tools;

        Self { max_iterations, max_history_messages, parallel_tools }
    }
}
