//! 从已启用插件中发现并加载技能目录。

use crate::app::agent::config::Config;
use crate::app::agent::skills::{Skill, SkillLoadMode, load_skills_from_directory};
use std::path::{Path, PathBuf};

use super::{PluginDiscoveryOptions, discover_plugins};

/// 从已启用插件的已审计技能目录加载技能。
///
/// 插件技能沿用统一加载器的模式与作用域规则，发现诊断不会被静默丢弃。
pub(crate) fn load_plugin_skills(
    workspace_dir: &Path,
    config: &Config,
    load_mode: SkillLoadMode,
) -> Vec<Skill> {
    let options = PluginDiscoveryOptions::new(
        workspace_dir.to_path_buf(),
        plugin_config_dir(config),
        config.plugins.clone(),
    );
    let discovery = match discover_plugins(&options) {
        Ok(discovery) => discovery,
        Err(error) => {
            tracing::warn!("skipping plugin skills: {error}");
            return Vec::new();
        }
    };

    discovery
        .enabled
        .into_iter()
        .filter_map(|plugin| {
            if plugin.manifest.skills.len() > 1 {
                tracing::warn!(
                    plugin = plugin.manifest.name,
                    "multiple plugin skill directories are not supported by the current loader"
                );
                return None;
            }
            plugin.manifest.skills.into_iter().next()
        })
        .flat_map(|skills_path| {
            load_skills_from_directory(
                &skills_path.canonical_path,
                load_mode,
                &config.skills.security,
            )
        })
        .collect()
}

fn plugin_config_dir(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
}
