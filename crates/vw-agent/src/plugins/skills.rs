//! Compose plugin skills with the active tenant turn selection.
use agent_config::runtime::Config;
use agent_skills::{Skill, types::SkillLoadMode};
use std::path::Path;
pub(crate) use agent_plugins::skills::catalog_skills;
#[cfg(test)]
pub(super) use agent_plugins::skills::PluginSkillInventory;
pub(crate) fn load_plugin_skills(workspace: &Path, config: &Config, mode: SkillLoadMode) -> Vec<Skill> {
    agent_plugins::skills::load_plugin_skills(workspace, config, mode, &super::compat::session_snapshot::discover_for_runtime)
}
#[cfg(test)]
pub(super) fn load_plugin_skill_inventory(workspace: &Path, config: &Config, mode: SkillLoadMode) -> PluginSkillInventory {
    agent_plugins::skills::load_plugin_skill_inventory(workspace, config, mode, &super::compat::session_snapshot::discover_for_runtime)
}
