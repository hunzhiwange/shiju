//! Gateway MCP runtime, global synchronization and project registration.
mod authorization;
use authorization as agent_apps;
pub mod host;
pub mod mcp;
pub mod global_sync;
pub mod project_mcp;
mod project_mcp_create;
mod project_mcp_update;
