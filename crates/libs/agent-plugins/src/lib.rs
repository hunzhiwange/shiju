//! Audited plugin paths and hook declaration loading.
pub mod audit;
pub mod hooks;
pub use plugin_types::{PluginHookSource, PluginPath};
pub use hooks::{PluginHook, PluginHookEvent, PluginHookMode, PluginHooksManifest, load_plugin_hook_source, load_plugin_hooks_manifest};

#[cfg(test)]
mod hooks_tests;
