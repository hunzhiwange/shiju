//! Persistence for the managed MCP catalog.

pub use agent_app_services::mcp::{credentials, McpCatalogStore, McpRevocation, StoredCatalogRecord};
#[cfg(not(target_arch = "wasm32"))]
pub use agent_app_services::mcp::{connection_runtime, probe_jobs_host, McpExecutionConnection};
#[cfg(target_os = "macos")]
pub use agent_app_services::mcp::{
    binding, bulk, data_migration, import, profile_slot, resync, probe_host,
    McpBindingTarget, McpImportEntry, McpImportOutcome, McpImportReport, McpImportSource,
    McpMigration, McpMigrationCommit, McpResyncResult, McpResyncTargetResult, McpResyncTask,
    resync_targets,
};

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

#[cfg(all(test, target_os = "macos"))]
use gateway_agent_apps::mcp_acceptance::catalog as catalog_create_tests;

#[cfg(all(test, target_os = "macos"))]
use gateway_agent_apps::mcp_acceptance::installation as binding_tests;
