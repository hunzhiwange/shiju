//! A standalone wire fixture: no tenant, OAuth, app grants or interactive capabilities.
use super::host::Host;
use agent_client_protocol::{McpServer, McpServerHttp, McpServerStdio};
use mcp_client::{FormOwner, RunningMcpPeer};
use plugin_mcp::{connection_host::{ConnectionHost, HostFuture}, mcp_connections::McpSessionConnections};
use std::{path::{Path, PathBuf}, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tool_types::Tool;
pub fn connections() -> Arc<dyn ConnectionHost> { Arc::new(WireHost) }
struct WireHost;
impl mcp_client::client::ClientHost for WireHost {
 fn runtime_stdio_cwd(&self, _: &McpServerStdio) -> anyhow::Result<Option<PathBuf>> { Ok(None) }
 fn http_headers<'a>(&'a self, _server: &'a McpServerHttp, _: Option<&'a FormOwner>) -> HostFuture<'a, Option<reqwest_012::header::HeaderMap>> { Box::pin(async move { Ok(None) }) }
 fn request_elicitation(&self, _: FormOwner, _: String, _: rmcp::model::CreateElicitationRequestParam, _: CancellationToken, _: CancellationToken, _: Option<serde_json::Value>) -> std::pin::Pin<Box<dyn Future<Output=Result<rmcp::model::CreateElicitationResult, rmcp::model::ErrorData>> + Send + '_>> { Box::pin(async { panic!("wire fixture cannot elicit") }) }
}
impl ConnectionHost for WireHost {
 fn request_abort(&self) -> anyhow::Result<Option<CancellationToken>> { Ok(None) }
 fn runtime_stdio_cwd(&self, _: &McpServerStdio) -> anyhow::Result<Option<PathBuf>> { Ok(None) }
 fn runtime_headers(&self, _: &McpServerHttp, _: Option<&str>) -> anyhow::Result<Option<reqwest_012::header::HeaderMap>> { anyhow::bail!("wire fixture only uses stdio") }
 fn oauth_access<'a>(&'a self, _: &'a McpServerHttp, _: &'a FormOwner) -> HostFuture<'a, Option<String>> { Box::pin(async { anyhow::bail!("wire fixture has no OAuth") }) }
 fn oauth_cancellation(&self, _: &McpServerHttp, _: &FormOwner) -> anyhow::Result<CancellationToken> { anyhow::bail!("wire fixture has no OAuth") }
 #[cfg(not(target_arch = "wasm32"))]
 fn selection_is_current(&self, _: &McpServer, _: Option<&vw_api_transport::TenantScope>) -> anyhow::Result<bool> { Ok(true) }
 fn open_peer(&self, server: McpServer, startup: Duration, closed: CancellationToken, owner: Option<FormOwner>) -> HostFuture<'_, RunningMcpPeer> { Box::pin(mcp_client::open_server_peer(Arc::new(WireHost), server, startup, closed, owner)) }
 fn discover<'a>(&'a self, server: McpServer, management: acp_config::McpServerManagement, existing_ids: &'a [&'a str], pool: Arc<McpSessionConnections>) -> HostFuture<'a, Vec<Arc<dyn Tool>>> { Box::pin(async move { Ok(crate::mcp::discover_dynamic_mcp_tools::<Host>(server, management, existing_ids, pool).await?.tools.into_iter().map(|tool| Arc::new(tool) as Arc<dyn Tool>).collect()) }) }
 fn revoke_app(&self, _: &vw_api_transport::TenantScope, _: &str, _: &Path) { panic!("wire fixture has no apps") }
}
impl tool_mcp::McpToolHost for Host {
 type Connections = McpSessionConnections;
 fn new_connections() -> Self::Connections { McpSessionConnections::new(connections()) }
 async fn load_config(_: Option<&Path>) -> anyhow::Result<acp_config::ResolvedAcpxConfig> { anyhow::bail!("fixture requires explicit configuration") }
 fn connection_required(server: &McpServer) -> anyhow::Result<bool> { Ok(plugin_mcp::mcp_connection_settings::settings(server)?.required) }
 fn missing_executable(_: &McpServer) -> anyhow::Result<Option<(String,String)>> { Ok(None) }
 fn tool_allowed(server: &McpServer, name: &str) -> anyhow::Result<bool> { plugin_mcp::mcp_connection_settings::tool_allowed(server,name) }
 fn tool_binding(pool: &Self::Connections, server: &McpServer) -> Option<mcp_client::ToolCatalogBinding> { pool.tool_binding(server) }
 async fn with_client<T: Send, F>(_: &McpServer, _: Option<Duration>, _: F) -> anyhow::Result<T> where F: for<'a> FnOnce(&'a RunningMcpPeer) -> tool_mcp::ClientFuture<'a,T> + Send { anyhow::bail!("fixture requires explicit connection pool") }
 async fn with_connection<T: Send,F>(pool: &Self::Connections, server: &McpServer, timeout: Option<Duration>, f: F) -> anyhow::Result<T> where F: for<'a> FnOnce(&'a RunningMcpPeer) -> tool_mcp::ClientFuture<'a,T> + Send { pool.with_client(server,timeout,f).await }
 fn app_resource<'a>(_: &'a RunningMcpPeer, _: &'a mut serde_json::Value) -> tool_mcp::ClientFuture<'a,()> { Box::pin(async { anyhow::bail!("wire fixture has no apps") }) }
 fn call_resource<'a>(_: &'a acp_config::ResolvedAcpxConfig, _: &'a Arc<Self::Connections>, _: serde_json::Value) -> tool_mcp::ClientFuture<'a,tool_types::ToolCallResult> { Box::pin(async { anyhow::bail!("wire fixture has no resources") }) }
}
