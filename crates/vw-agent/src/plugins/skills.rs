//! Runtime loading for skills packaged in enabled plugins.

use crate::app::agent::config::Config;
use crate::app::agent::skills::{Skill, SkillLoadMode, load_skills_from_directory};
use std::path::{Path, PathBuf};

use super::{PluginDiscoveryOptions, discover_plugins};

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
        .filter_map(|plugin| plugin.manifest.skills)
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
