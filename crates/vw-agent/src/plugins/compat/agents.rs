//! Host path auditing is injected without changing its safety policy.
pub use plugin_install::agents::LoadedAgentMetadata;
use std::path::Path;
pub(crate) fn load_metadata(plugin_root: &Path, skill_dir: &Path) -> LoadedAgentMetadata {
    plugin_install::agents::load_metadata(plugin_root, skill_dir, super::super::audit::resolve_component_path)
}
