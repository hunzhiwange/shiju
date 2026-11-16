//! MCP inspection, status, resource and dynamic tool execution.
pub mod mcp_inspect;
pub mod mcp_resource;
pub mod mcp_status;
pub mod mcp_tool;
mod host;
pub use host::{McpToolHost, ClientFuture};

#[cfg(test)]
mod host_migration_35_tests;

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod host_migration_35_acceptance;
