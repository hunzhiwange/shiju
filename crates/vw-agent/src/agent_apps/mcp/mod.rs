//! Persistence for the managed MCP catalog.

pub use agent_app_services::mcp::*;

#[cfg(not(target_arch = "wasm32"))]
mod connectivity;
#[cfg(target_os = "macos")]
pub mod network_probe;
#[cfg(not(target_arch = "wasm32"))]
pub mod probe_jobs;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod revocation;
#[cfg(all(test, target_os = "macos"))]
mod revocation_tests;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod runtime;
#[cfg(all(test, target_os = "macos"))]
mod runtime_tests;

pub(crate) mod acp_mount;
#[cfg(all(test, target_os = "macos"))]
mod acp_mount_tests;


#[cfg(all(test, target_os = "macos"))]
mod security_tests;

#[cfg(all(test, target_os = "macos"))]
mod host_scope_tests;

#[cfg(test)]
#[path = "catalog_test_support.rs"]
mod catalog_create_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "binding_test_support.rs"]
mod binding_tests;
