//! Trusted capability policy resolution and tool composition.
use tool_types::Tool;
use config_agent::DelegateAgentConfig;
use crate::capability_policy::resolve_skill_capabilities;
use std::sync::Arc;
pub fn build_agentic_tools(parent_tools: &[Arc<dyn Tool>], agent_config: &DelegateAgentConfig, trusted_user_selected_skills: &[String]) -> Vec<Box<dyn Tool>> {
    let skill_capabilities = resolve_skill_capabilities(
        agent_config.allowed_skills.iter().chain(trusted_user_selected_skills.iter()),
        &agent_config.allowed_skills,
        trusted_user_selected_skills,
        agent_config.skill_policy,
        std::iter::empty::<&str>(),
    );
    tool_services::delegated_tools::build_agentic_tools(parent_tools, agent_config, skill_capabilities.visible_ids)
}
