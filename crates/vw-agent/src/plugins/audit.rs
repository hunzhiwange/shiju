//! 插件路径审计的兼容入口。
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(super) use agent_plugins::audit::resolve_component_path;
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use agent_plugins::audit::{canonical_plugin_root, plugin_manifest_path, resolve_component_path};
