//! Local/plugin skill loading, project selection and catalog precedence.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use config_skills::{SkillsConfig, SkillsDirectoryProvider, SkillSecurityConfig};
use crate::{Skill, types::SkillLoadMode, loader};
use crate::local_sources::{discover_local_skill_source_dirs_with_provider, LocalSkillSourceKind};

fn extend_unique_skills(
    target: &mut Vec<Skill>,
    seen: &mut HashSet<(Option<String>, String)>,
    items: Vec<Skill>,
) {
    for skill in items {
        let key = match &skill.plugin_source {
            Some(source) => {
                // 项目同名技能仍优先；不同插件的同名技能保留各自身份供调用层消歧。
                if seen.contains(&(None, skill.name.clone())) {
                    tracing::warn!(
                        plugin = source.plugin_id,
                        skill = skill.name,
                        "plugin skill shadowed by a local skill"
                    );
                    continue;
                }
                (Some(source.plugin_id.clone()), skill.name.clone())
            }
            None => (None, skill.name.clone()),
        };
        if seen.insert(key) {
            target.push(skill);
        }
    }
}

fn load_workspace_skills(
    workspace_dir: &Path,
    load_mode: SkillLoadMode,
    directory_provider: SkillsDirectoryProvider,
    security: &config_skills::SkillSecurityConfig,
) -> Vec<Skill> {
    discover_local_skill_source_dirs_with_provider(Some(workspace_dir), directory_provider)
        .into_iter()
        .flat_map(|source| loader::load_skills_from_directory(&source.path, load_mode, security))
        .collect()
}

pub fn load_project_skills_with_config(
    workspace_dir: &Path,
    config: &config_skills::SkillsConfig,
) -> Vec<Skill> {
    let mut skills = Vec::new();
    let mut seen = HashSet::new();
    let load_mode = SkillLoadMode::from_prompt_mode(config.prompt_injection_mode);

    for source in discover_local_skill_source_dirs_with_provider(
        Some(workspace_dir),
        config.directory_provider,
    )
    .into_iter()
    .filter(|source| {
        matches!(source.kind, LocalSkillSourceKind::Workspace | LocalSkillSourceKind::Ancestor)
    }) {
        extend_unique_skills(
            &mut skills,
            &mut seen,
            loader::load_skills_from_directory(&source.path, load_mode, &config.security),
        );
    }

    skills
}

/// Load legacy default local sources without querying plugin services.
pub fn load_skills(workspace: &Path) -> Vec<Skill> {
    load_with_provider(
        workspace,
        SkillLoadMode::Full,
        SkillsDirectoryProvider::Vibewindow,
        &SkillSecurityConfig::default(),
        |_| Vec::new(),
    )
}

/// Query plugin inventory only after local discovery, then apply local precedence.
pub fn load_skills_with_config(
    workspace: &Path,
    config: &SkillsConfig,
    full: bool,
    plugin_skills: impl FnOnce(SkillLoadMode) -> Vec<Skill>,
) -> Vec<Skill> {
    let mode = if full {
        SkillLoadMode::Full
    } else {
        SkillLoadMode::from_prompt_mode(config.prompt_injection_mode)
    };
    load_with_provider(workspace, mode, config.directory_provider, &config.security, plugin_skills)
}

fn load_with_provider(
    workspace: &Path,
    mode: SkillLoadMode,
    provider: SkillsDirectoryProvider,
    security: &SkillSecurityConfig,
    plugin_skills: impl FnOnce(SkillLoadMode) -> Vec<Skill>,
) -> Vec<Skill> {
    let mut skills = Vec::new();
    let mut seen = HashSet::new();
    extend_unique_skills(
        &mut skills,
        &mut seen,
        load_workspace_skills(workspace, mode, provider, security),
    );
    extend_unique_skills(&mut skills, &mut seen, plugin_skills(mode));
    skills
}

pub fn skills_dir(workspace: &Path) -> PathBuf {
    workspace.join("skills")
}
