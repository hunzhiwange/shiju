//! Domain ownership of the former host skill scenarios.
use crate::{Skill, catalog::{load_skills, skills_dir}, invoked::InvokedSkillRecord,
    session_state::SkillListingSessionState, local_sources::{LocalSkillSourceKind,
    discover_local_skill_source_dirs, workspace_skills_dir}};
use config_skills::SkillsDirectoryProvider;
use crate::init::init_skills_dir;
use crate::installer::InstallResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::cli::print_install_result as print_install_success;

// Minimal command input fixture; no host configuration or runtime state.
#[derive(Default)]
struct Config {
    workspace_dir: std::path::PathBuf,
    skills: config_skills::SkillsConfig,
}
fn load_skills_with_config(workspace: &std::path::Path, config: &Config) -> Vec<Skill> {
    crate::catalog::load_skills_with_config(workspace, &config.skills, false, |_| Vec::new())
}
fn load_skills_full_with_config(workspace: &std::path::Path, config: &Config) -> Vec<Skill> {
    crate::catalog::load_skills_with_config(workspace, &config.skills, true, |_| Vec::new())
}
fn handle_command(command: skill_core::types::SkillCommands, config: &Config) -> anyhow::Result<()> {
    crate::cli::handle_command(command, &config.workspace_dir, &config.skills, |_| Vec::new())
}
mod cli_tests;
mod init_tests;
mod installer_tests;
mod mod_tests;
mod discovery_tests;
mod listing_tests;
mod invoked_tests;
mod symlink_tests;
mod tests;
