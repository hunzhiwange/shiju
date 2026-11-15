//! Delegated tool availability and limits, without expanding strict defaults.
use config_agent::{DelegateAgentConfig, merged_agent_configs};
use std::collections::HashMap;

const DEFAULT_AGENT_TOOL_PARALLEL_LIMIT: usize = 4;
const MAX_AGENT_TOOL_PARALLEL_LIMIT: usize = 32;

pub fn agent_tool_parallel_limit(
    agents: &HashMap<String, DelegateAgentConfig>,
    agent_name: &str,
) -> usize {
    agents
        .get(agent_name)
        .map(|config| config.max_parallel_subagents)
        .unwrap_or(DEFAULT_AGENT_TOOL_PARALLEL_LIMIT)
        .clamp(1, MAX_AGENT_TOOL_PARALLEL_LIMIT)
}

pub fn enabled_delegate_agents(
    agents: &HashMap<String, DelegateAgentConfig>,
) -> HashMap<String, DelegateAgentConfig> {
    merged_agent_configs(agents)
        .into_iter()
        .filter(|(_, config)| config.enabled && config.allow_as_subagent)
        .collect()
}

pub fn extend_project_skills(
    enabled: &mut HashMap<String, DelegateAgentConfig>,
    project_skill_names: &[String],
) {
    for config in enabled.values_mut() {
        // Defaults must never expand an explicitly strict access boundary.
        if config.skill_policy == config_agent::DelegateCapabilityPolicy::Strict
            || matches!(config.defaults.skills, config_agent_profile::AgentDefault::Fixed(_))
        {
            continue;
        }
        config.allowed_skills.extend(project_skill_names.iter().cloned());
        config.allowed_skills.sort();
        config.allowed_skills.dedup();
    }

}
