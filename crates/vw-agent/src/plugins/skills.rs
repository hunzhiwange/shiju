//! 从已启用插件的全部已审计技能根加载技能并保留来源诊断。

use crate::app::agent::config::Config;
use crate::app::agent::skills::{PluginSkillSource, Skill, SkillLoadMode, load_skill_directory};
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::audit::resolve_component_path;
use super::compat::skill_frontmatter;
use super::compat::skill_roots::{SkillPackageFormat, package_format, skill_files, skill_roots};
use super::{DiscoveredPlugin, PluginDiscoveryOptions, discover_plugins};

#[derive(Debug)]
pub(super) struct PluginSkillDiagnostic {
    pub plugin_id: Option<String>,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Default)]
pub(super) struct PluginSkillInventory {
    pub skills: Vec<Skill>,
    pub diagnostics: Vec<PluginSkillDiagnostic>,
}

/// 运行时入口复用结构化清单；未能加载的技能与冲突不会静默消失。
pub(crate) fn load_plugin_skills(
    workspace_dir: &Path,
    config: &Config,
    load_mode: SkillLoadMode,
) -> Vec<Skill> {
    let inventory = load_plugin_skill_inventory(workspace_dir, config, load_mode);
    for diagnostic in inventory.diagnostics {
        tracing::warn!(plugin = ?diagnostic.plugin_id, path = %diagnostic.path.display(), error = diagnostic.message, "plugin skill diagnostic");
    }
    inventory.skills
}

pub(super) fn load_plugin_skill_inventory(
    workspace_dir: &Path,
    config: &Config,
    load_mode: SkillLoadMode,
) -> PluginSkillInventory {
    let options = PluginDiscoveryOptions::new(
        workspace_dir.to_path_buf(),
        plugin_config_dir(config),
        config.plugins.clone(),
    );
    let discovery = match discover_plugins(&options) {
        Ok(discovery) => discovery,
        Err(error) => {
            return PluginSkillInventory {
                skills: Vec::new(),
                diagnostics: vec![PluginSkillDiagnostic {
                    plugin_id: None,
                    path: workspace_dir.to_path_buf(),
                    message: format!("{error:#}"),
                }],
            };
        }
    };
    let mut inventory = PluginSkillInventory {
        skills: Vec::new(),
        diagnostics: discovery
            .diagnostics
            .into_iter()
            .map(|diagnostic| PluginSkillDiagnostic {
                plugin_id: None,
                path: diagnostic.plugin_root,
                message: diagnostic.message,
            })
            .collect(),
    };
    for plugin in discovery.enabled {
        load_plugin(&plugin, config, load_mode, &mut inventory);
    }
    inventory
}

fn load_plugin(
    plugin: &DiscoveredPlugin,
    config: &Config,
    mode: SkillLoadMode,
    inventory: &mut PluginSkillInventory,
) {
    let format = package_format(&plugin.manifest);
    let mut names = HashMap::<String, PathBuf>::new();
    let roots = match skill_roots(&plugin.manifest) {
        Ok(roots) => roots,
        Err(error) => {
            diagnostic(inventory, plugin, &plugin.manifest.root, format!("{error:#}"));
            load_commands(plugin, config, mode, inventory, &mut names);
            return;
        }
    };
    let mut visited = HashSet::new();
    // Use Codex's logical migration root for precedence, never the physical host cache path.
    let command_root = plugin.manifest.root.join(".codex-plugin/migrated-command-skills");
    let mut commands_loaded = false;
    for root in roots {
        if !commands_loaded && root >= command_root {
            load_commands(plugin, config, mode, inventory, &mut names);
            commands_loaded = true;
        }
        let (files, errors) = skill_files(&root, format);
        for (path, message) in errors {
            diagnostic(inventory, plugin, &path, message);
        }
        for file in files {
            let directory = file.parent().expect("discovered skill has a parent");
            if !visited.insert(directory.to_path_buf()) {
                continue;
            }
            match load_candidate(plugin, directory, format, config, mode) {
                Ok(Some(skill)) => {
                    if let Some(winner) = names.get(&skill.name) {
                        diagnostic(
                            inventory,
                            plugin,
                            &file,
                            format!(
                                "duplicate skill name {:?}; keeping {}",
                                skill.name,
                                winner.display()
                            ),
                        );
                    } else {
                        names.insert(
                            skill.name.clone(),
                            skill
                                .location
                                .clone()
                                .expect("loaded plugin skill has an audited entry"),
                        );
                        inventory.skills.push(skill);
                    }
                }
                Ok(None) => {}
                Err(error) => diagnostic(inventory, plugin, &file, format!("{error:#}")),
            }
        }
    }
    if !commands_loaded {
        load_commands(plugin, config, mode, inventory, &mut names);
    }
}

fn load_commands(
    plugin: &DiscoveredPlugin,
    config: &Config,
    mode: SkillLoadMode,
    inventory: &mut PluginSkillInventory,
    names: &mut HashMap<String, PathBuf>,
) {
    let commands = super::compat::commands::load_commands(plugin, config, mode);
    for (path, message) in commands.diagnostics {
        diagnostic(inventory, plugin, &path, message);
    }
    for skill in commands.skills {
        if names.contains_key(&skill.name) {
            diagnostic(
                inventory,
                plugin,
                skill.location.as_ref().expect("cached command location"),
                format!(
                    "duplicate skill name {:?}; keeping earlier logical skill root",
                    skill.name
                ),
            );
        } else {
            names.insert(
                skill.name.clone(),
                skill.location.clone().expect("cached command location"),
            );
            inventory.skills.push(skill);
        }
    }
}

fn load_candidate(
    plugin: &DiscoveredPlugin,
    directory: &Path,
    format: SkillPackageFormat,
    config: &Config,
    mode: SkillLoadMode,
) -> anyhow::Result<Option<Skill>> {
    // Reuse the manifest boundary for the actual entry, including every parent component.
    let entry = if format == SkillPackageFormat::Native && directory.join("SKILL.toml").exists() {
        directory.join("SKILL.toml")
    } else {
        directory.join("SKILL.md")
    };
    let relative = entry.strip_prefix(&plugin.manifest.root)?;
    let components = relative
        .iter()
        .map(|part| part.to_str().context("plugin skill path must be UTF-8"))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let raw = format!("./{}", components.join("/"));
    let entry = resolve_component_path(&plugin.manifest.root, "skill entry", &raw)?.canonical_path;
    let Some(mut skill) = load_skill_directory(
        directory,
        mode,
        &config.skills.security,
        format != SkillPackageFormat::Native,
    )?
    else {
        return Ok(None);
    };
    if format != SkillPackageFormat::Native {
        let content = std::fs::read_to_string(&entry)?;
        let default_name = directory.file_name().and_then(|name| name.to_str()).unwrap_or("");
        let metadata = skill_frontmatter::parse(&content, default_name)?;
        skill.name = metadata.name;
        skill.description = metadata.description;
        skill.short_description = metadata.short_description;
    }
    skill.location = Some(entry.clone());
    skill.plugin_source = Some(PluginSkillSource {
        plugin_id: plugin.identity.as_key(),
        namespace: plugin.manifest.name.clone(),
        root: plugin.manifest.root.clone(),
        relative_skill_path: entry.strip_prefix(&plugin.manifest.root)?.to_path_buf(),
    });
    Ok(Some(skill))
}

fn diagnostic(
    inventory: &mut PluginSkillInventory,
    plugin: &DiscoveredPlugin,
    path: &Path,
    message: String,
) {
    inventory.diagnostics.push(PluginSkillDiagnostic {
        plugin_id: Some(plugin.identity.as_key()),
        path: path.to_path_buf(),
        message,
    });
}

fn plugin_config_dir(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
}
