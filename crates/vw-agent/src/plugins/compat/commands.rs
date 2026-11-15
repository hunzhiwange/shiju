//! Compose the command migration with the existing package classification and path audit.
pub use plugin_install::commands::CommandInventory;
use crate::{config::Config, plugins::DiscoveredPlugin, skills::SkillLoadMode};
pub(crate) fn load_commands(plugin: &DiscoveredPlugin, config: &Config, mode: SkillLoadMode) -> CommandInventory {
    plugin_install::commands::load_commands(
        &plugin.manifest, &plugin.identity, &config.config_path, &config.skills.security, mode,
        super::skill_roots::package_format(&plugin.manifest) == super::skill_roots::SkillPackageFormat::Legacy,
        &super::super::audit::resolve_component_path,
    )
}
