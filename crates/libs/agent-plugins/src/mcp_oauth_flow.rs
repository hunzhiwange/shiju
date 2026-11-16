//! Host source adapter for the plugin-owned authorization flow.
pub use plugin_mcp::mcp_oauth_flow::*;
pub async fn authorize(server: &agent_client_protocol::McpServerHttp, session: String, tenant: vw_api_transport::TenantScope, cancelled: tokio_util::sync::CancellationToken) -> anyhow::Result<String> {
    plugin_mcp::mcp_oauth_flow::authorize(&super::mcp_auth::Source, server, session, tenant, cancelled).await
}
