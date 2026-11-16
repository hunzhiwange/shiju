//! Package compatibility contracts, using real domain discovery and invocation.
mod host;
type SkillTool = tool_services::skill::SkillTool<host::Host>;
type ToolUseContext = tool_context::context::ToolUseContext<host::Host>;
use tool_context::context::scope_tool_use_context;
use tool_services::skill;
fn load_skills(workspace: &std::path::Path, config: &agent_config::runtime::Config, full: bool) -> Vec<agent_skills::Skill> {
 agent_skills::catalog::load_skills_with_config(workspace, &config.skills, full, |mode| crate::skills::load_plugin_skills(workspace, config, mode, &crate::discover_plugins))
}
fn load_skills_full_with_config(workspace: &std::path::Path, config: &agent_config::runtime::Config) -> Vec<agent_skills::Skill> { load_skills(workspace, config, true) }
mod skills {
 pub use agent_skills::{Skill, types::SkillLoadMode, session_state::SkillListingSessionState};
 pub(super) use super::load_skills_full_with_config;
}
fn load_plugin_skill_inventory(workspace: &std::path::Path, config: &agent_config::runtime::Config, mode: agent_skills::types::SkillLoadMode) -> crate::skills::PluginSkillInventory {
 crate::skills::load_plugin_skill_inventory(workspace, config, mode, &crate::discover_plugins)
}
fn onboarding_entry(manifest: &crate::PluginManifest, identity: &crate::PluginIdentity, config: &agent_config::runtime::Config) -> anyhow::Result<Option<String>> {
 crate::onboarding::entry(manifest, identity, &load_skills_full_with_config(&config.workspace_dir, config))
}
fn installation_message(result: &crate::PluginInstallResult, config: &agent_config::runtime::Config) -> String {
 crate::cli::installation_message(result, config, &|config| load_skills_full_with_config(&config.workspace_dir, config))
}
mod t09_tests;
mod t10_tests;
mod t11_tests;
mod t12_tests;
mod t13_tests;
mod t66_tests;
mod closure_tests;

fn set_plugin_enabled(dir: &std::path::Path, name: &str, enabled: bool) -> anyhow::Result<()> { crate::install::set_plugin_enabled(dir, name, enabled, &|_| {}) }
