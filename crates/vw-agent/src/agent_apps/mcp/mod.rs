//! Persistence for the managed MCP catalog.

pub use agent_app_services::mcp::*;

#[cfg(all(test, target_os = "macos"))]
mod connection_probe_tests;
#[cfg(not(target_arch = "wasm32"))]
mod connectivity;
#[cfg(target_os = "macos")]
pub mod network_probe;
#[cfg(all(test, target_os = "macos"))]
mod network_probe_tests;
#[cfg(not(target_arch = "wasm32"))]
pub mod probe_jobs;
#[cfg(all(test, target_os = "macos"))]
mod probe_jobs_tests;
#[cfg(all(test, target_os = "macos"))]
mod stdio_probe_tests;

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
mod tool_details_tests;

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
