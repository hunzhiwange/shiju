//! File HTTP routes and workspace-safe local copy operations.
#![cfg(not(target_arch = "wasm32"))]
mod instance_support;
mod file;
pub use instance_support::Host;
pub use file::router;
