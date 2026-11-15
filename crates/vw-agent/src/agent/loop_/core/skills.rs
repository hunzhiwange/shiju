//! 宿主技能能力绑定；状态与规则唯一归属 agent-loop。
use agent_loop::core::skills::CoreSkillHost;
use crate::{config::Config, skills::{Skill, InvokedSkillRecord}, tools::ToolUseContext};
use std::{path::Path, collections::BTreeSet};
use serde_json::Value;
#[derive(Clone)]
pub struct NativeCoreSkillHost;
pub type CoreSkillSession = agent_loop::core::skills::CoreSkillSession<NativeCoreSkillHost>;
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl CoreSkillHost for NativeCoreSkillHost {
    type Context = crate::tools::context::AgentContextHost;
    fn current_tenant_scope() -> vw_api_transport::id::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn load_skills_with_config(directory: &Path, config: &Config) -> Vec<Skill> { crate::skills::load_skills_with_config(directory, config) }
    async fn get_model(provider: &str, model: &str) -> Result<model_types::Model, agent_provider::provider::CatalogError> { crate::provider::provider::get_model(provider, model).await }
    fn explicitly_referenced_skill_names(skills: &[Skill], text: &str) -> BTreeSet<String> { crate::tools::skill::explicitly_referenced_skill_names(skills, [text]) }
    fn user_explicitly_requests_skill_use(text: &str) -> bool { crate::tools::skill::user_explicitly_requests_skill_use(text) }
    fn user_invocable_skill_names(skills: &[Skill]) -> BTreeSet<String> { crate::tools::skill::user_invocable_skill_names(skills) }
    #[cfg(not(target_arch = "wasm32"))]
    fn accessible_skills(skills: Vec<Skill>, context: Option<&ToolUseContext>) -> Vec<Skill> { crate::tools::skill::accessible_skills(skills, context) }
    fn description_for_skills(directory: &Path, skills: Vec<Skill>) -> String { crate::tools::skill::description_for_skills(directory, skills) }
    async fn restore(session: &str, agent: &str, history: &[Value]) -> Result<Vec<InvokedSkillRecord>, String> { crate::session::skill_checkpoint::restore(session, agent, history).await }
    async fn save(session: &str, agent: &str, history: &[Value], records: &[InvokedSkillRecord]) -> Result<(), String> { crate::session::skill_checkpoint::save(session, agent, history, records).await }
}
