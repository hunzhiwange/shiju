//! Compatibility export; implementation belongs to plugin-hooks.
#[cfg(any(test, target_arch = "wasm32"))]
pub use plugin_hooks::hook_matcher::*;
