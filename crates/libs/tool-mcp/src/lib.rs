//! MCP inspection, status, resource and dynamic tool execution.
pub mod mcp_inspect;
pub mod mcp_resource;
pub mod mcp_status;
pub mod mcp_tool;
mod host;
pub use host::{McpToolHost, ClientFuture};
