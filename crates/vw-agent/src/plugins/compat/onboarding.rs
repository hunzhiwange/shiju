//! 安装引导仅提供普通技能调用入口，不在安装期间执行技能。

use anyhow::{Result, bail};

use crate::config::Config;
use crate::skills::load_skills_full_with_config;

use super::super::audit::resolve_component_path;
use super::super::{PluginIdentity, PluginManifest};

pub(crate) fn entry(
    manifest: &PluginManifest,
    identity: &PluginIdentity,
    config: &Config,
) -> Result<Option<String>> {
    let Some(declared) = &manifest.onboarding_skill else {
        return Ok(None);
    };
    // Reaudit the selected file, then use the same admitted inventory as ordinary skill calls.
    // A contained file, extra skill root or another installation with the same name is insufficient.
    let declared = resolve_component_path(&manifest.root, "onboardingSkill", &declared.raw)?;
    let skills = load_skills_full_with_config(&config.workspace_dir, config);
    let key = identity.as_key();
    let Some(skill) = skills.iter().find(|skill| {
        skill.location.as_ref() == Some(&declared.canonical_path)
            && skill
                .plugin_source
                .as_ref()
                .is_some_and(|source| source.plugin_id == key && source.root == manifest.root)
    }) else {
        bail!("declared onboardingSkill is not an enabled, audited package skill");
    };
    if !skill.user_invocable {
        bail!("declared onboardingSkill does not allow explicit user invocation");
    }
    let name = skill.invocation_name();
    if skills.iter().filter(|skill| skill.invocation_aliases().contains(&name)).take(2).count() > 1
    {
        bail!("declared onboardingSkill has an ambiguous ordinary skill invocation name");
    }
    Ok(Some(name))
}
