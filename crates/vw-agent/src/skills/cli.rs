//! Host inventory adapter for skill commands.
pub fn handle_command(
    command: crate::skill::SkillCommands,
    config: &crate::config::Config,
) -> anyhow::Result<()> {
    agent_skills::cli::handle_command(command, &config.workspace_dir, &config.skills, |mode| {
        crate::plugins::load_plugin_skills(&config.workspace_dir, config, mode)
    })
}
