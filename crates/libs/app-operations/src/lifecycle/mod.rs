//! Lifecycle recipes and local execution exports.
pub use app_lifecycle::*;
#[cfg(unix)]
pub use app_uninstall::local;
