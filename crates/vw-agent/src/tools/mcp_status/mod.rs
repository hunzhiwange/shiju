//! MCP status tool composed with scoped configuration and connections.
pub type McpStatusTool = tool_mcp::mcp_status::McpStatusTool<super::context::AgentContextHost>;
// These tests exercise the concrete AgentContextHost alias, scoped config loading,
// registry discovery and real MCP connections. Their source lives beside tool-mcp,
// but this host owns the test target; do not also register it in tool-mcp.
#[cfg(test)]
#[path = "../../../../libs/tool-mcp/src/mcp_status/tests.rs"]
mod tests;
