//! 组合插件命令、可信技能库存和原租户连接撤销动作。
fn load_skills(config: &crate::config::Config) -> Vec<agent_skills::Skill> { crate::skills::load_skills_full_with_config(&config.workspace_dir, config) }
pub fn handle_command(command: super::PluginCommands, config: &crate::config::Config) -> anyhow::Result<()> {
    agent_plugins::cli::handle_command(command, config, &super::compat::mcp_connection_sessions::close_plugin, &load_skills)
}
pub(super) fn installation_message(result: &super::PluginInstallResult, config: &crate::config::Config) -> String {
    agent_plugins::cli::installation_message(result, config, &load_skills)
}
