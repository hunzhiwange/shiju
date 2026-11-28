//! Persistence for the managed MCP catalog.

pub use agent_app_services::mcp::{credentials, McpCatalogStore, McpRevocation, StoredCatalogRecord};
#[cfg(not(target_arch = "wasm32"))]
pub use agent_app_services::mcp::McpExecutionConnection;
#[cfg(target_os = "macos")]
pub use agent_app_services::mcp::{
    binding, profile_slot, McpBindingTarget,
};

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
use gateway_agent_apps::mcp_acceptance::catalog as catalog_create_tests;

#[cfg(all(test, target_os = "macos"))]
use gateway_agent_apps::mcp_acceptance::installation as binding_tests;
