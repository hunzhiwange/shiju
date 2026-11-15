//! Managed MCP HTTP routes backed by domain services.
use app_access_service::access as agent_apps;
mod mcp_management_bindings;
mod mcp_management_create;
mod mcp_management_delete;
mod mcp_management_import;
pub mod mcp_management_read;
pub mod mcp_management_resync;
mod mcp_management_update;
pub mod host;
pub mod mcp;
#[cfg(target_os = "macos")]
pub mod mcp_management_tools;
#[cfg(target_os = "macos")]
pub mod mcp_management_probe;
