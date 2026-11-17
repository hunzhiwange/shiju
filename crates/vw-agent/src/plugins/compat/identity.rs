//! Implementation owned by plugin-install.
#[cfg(any(test, target_arch = "wasm32"))]
pub use plugin_install::identity::*;
