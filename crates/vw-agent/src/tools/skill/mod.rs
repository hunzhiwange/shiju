//! Concrete skill-tool binding; business implementation belongs to tool-services.
pub type SkillTool = tool_services::skill::SkillTool<super::context::AgentContextHost>;
pub use tool_services::skill::{description_for_skills, explicitly_referenced_skill_names,
    user_explicitly_requests_skill_use, user_invocable_skill_names};
#[cfg(not(target_arch = "wasm32"))]
pub fn accessible_skills(skills: Vec<crate::skills::Skill>, context: Option<&super::context::ToolUseContext>) -> Vec<crate::skills::Skill> {
    tool_services::skill::accessible_skills::<super::context::AgentContextHost>(skills, context)
}

#[cfg(test)]
use tool_services::skill::{Args, available_skill_names, base_dir_url, file_url,
    skill_file_id, skill_is_explicitly_requested, skill_location, skill_matches_name,
    skill_prompt_content, text_contains_skill_reference, xml_escape, skill_permission_rule};
#[cfg(test)]
use tool_services::skill::invocation;
#[cfg(test)]
fn block_on<F: std::future::Future + Send>(future: F) -> F::Output where F::Output: Send {
    tool_services::skill::block_on::<super::context::AgentContextHost, _>(future)
}
#[cfg(test)]
fn filter_delegate_skills(skills: Vec<crate::skills::Skill>) -> Vec<crate::skills::Skill> {
    tool_services::skill::filter_delegate_skills::<super::context::AgentContextHost>(skills)
}
#[cfg(test)]
fn skill_request_is_explicit(name: &str) -> bool {
    tool_services::skill::skill_request_is_explicit::<super::context::AgentContextHost>(name)
}
#[cfg(test)]
mod prompt {
    pub fn build_description_text(path: &std::path::Path, config: &crate::config::Config) -> String {
        tool_services::skill::prompt::build_description_text::<super::super::context::AgentContextHost>(path, config)
    }
}

#[cfg(test)]
#[path = "../../../../libs/tool-services/src/skill/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../../libs/tool-services/src/skill/mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "../../../../libs/tool-services/src/skill/invocation_tests.rs"]
mod invocation_tests;
