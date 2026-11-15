//! Compose skill root discovery with the existing audited path resolver.
pub(crate) use plugin_install::skill_roots::{SkillPackageFormat, package_format, skill_files};
pub(crate) fn skill_roots(manifest: &super::super::PluginManifest) -> anyhow::Result<Vec<std::path::PathBuf>> {
    plugin_install::skill_roots::skill_roots(manifest, &super::super::audit::resolve_component_path)
}
