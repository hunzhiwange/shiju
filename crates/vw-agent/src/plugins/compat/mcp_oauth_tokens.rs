//! Host source adapter; encrypted grant state belongs to plugin-mcp.
pub use plugin_mcp::mcp_oauth_tokens::*;
pub(crate) async fn access(server: &agent_client_protocol::McpServerHttp, owner: &super::mcp_elicitation::FormOwner) -> anyhow::Result<Option<String>> {
    plugin_mcp::mcp_oauth_tokens::access(&super::mcp_auth::Source, server, owner).await
}
#[cfg(test)]
pub(super) fn save(server: &agent_client_protocol::McpServerHttp, owner: &super::mcp_elicitation::FormOwner, grant: &Grant) -> anyhow::Result<()> {
    plugin_mcp::mcp_oauth_tokens::save(&super::mcp_auth::Source, server, owner, grant)
}
