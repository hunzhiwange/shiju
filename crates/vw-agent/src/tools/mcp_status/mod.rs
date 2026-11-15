//! MCP status tool composed with scoped configuration and connections.
pub type McpStatusTool = tool_mcp::mcp_status::McpStatusTool<super::context::AgentContextHost>;
#[cfg(test)]
#[path = "../../../../libs/tool-mcp/src/mcp_status/tests.rs"]
mod tests;
