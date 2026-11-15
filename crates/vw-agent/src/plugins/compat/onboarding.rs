//! 组合宿主可信技能库存与库内onboarding审计。
pub(crate) fn entry(manifest: &crate::plugins::PluginManifest, identity: &crate::plugins::PluginIdentity, config: &crate::config::Config) -> anyhow::Result<Option<String>> {
    if manifest.onboarding_skill.is_none() { return Ok(None); }
    let skills = crate::skills::load_skills_full_with_config(&config.workspace_dir, config);
    agent_plugins::onboarding::entry(manifest, identity, &skills)
}
