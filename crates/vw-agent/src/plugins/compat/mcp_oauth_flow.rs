//! Host source adapter for the plugin-owned authorization flow.
pub use plugin_mcp::mcp_oauth_flow::*;
pub async fn authorize(server: &agent_client_protocol::McpServerHttp, session: String, tenant: vw_api_transport::TenantScope, cancelled: tokio_util::sync::CancellationToken) -> anyhow::Result<String> {
    plugin_mcp::mcp_oauth_flow::authorize(&super::mcp_auth::Source, server, session, tenant, cancelled).await
}
#[cfg(test)]
pub(super) async fn authorize_with<D, X, XF>(server: &agent_client_protocol::McpServerHttp, owner: super::mcp_elicitation::FormOwner, cancelled: tokio_util::sync::CancellationToken, timeout: std::time::Duration, discovery: D, exchange: X) -> anyhow::Result<String>
where D: std::future::Future<Output = anyhow::Result<super::mcp_oauth_discovery::OAuthDiscovery>>,
X: FnMut(url::Url, String, &'static str) -> XF,
XF: std::future::Future<Output = anyhow::Result<serde_json::Value>> {
    plugin_mcp::mcp_oauth_flow::authorize_with(&super::mcp_auth::Source, server, owner, cancelled, timeout, discovery, exchange).await
}
