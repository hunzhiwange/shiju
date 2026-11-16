//! Package discovery and real MCP transports without application state.
use agent_client_protocol::{McpServer, McpServerHttp, McpServerStdio};
use anyhow::Result;
use std::{future::Future, path::{Path, PathBuf}, pin::Pin, sync::Arc};
use tokio_util::sync::CancellationToken;

mod t07_tests;
mod t15_tests;
mod t16_tests;
mod t17_tests;
mod t17_public_tests;
mod t26_tests;

struct Source;
impl plugin_mcp::source_host::SourceHost for Source {
    fn load_manifest(&self, root: &Path) -> Result<plugin_types::PluginManifest> {
        crate::load_plugin_manifest(root)
    }
    fn load_declarations(&self, manifest: &plugin_types::PluginManifest)
        -> Result<(plugin_mcp::mcp_legacy::PluginMcpParseOutcome, PathBuf)> {
        crate::mcp::load_plugin_mcp_declarations(manifest)
    }
}

fn merge_enabled_plugin_mcp_servers(servers: &mut Vec<McpServer>, options: &crate::PluginDiscoveryOptions) -> Result<()> {
    crate::mcp::merge_enabled_plugin_mcp_servers(servers, options, &|options| {
        Ok((crate::discover_plugins(options)?, options.clone()))
    })
}

// Exercise the actual audited directory/header readers and actual protocol client.
// These tests have no model turn, app selection, or session registry privileges.
struct Client;
impl mcp_client::client::ClientHost for Client {
    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> Result<Option<PathBuf>> {
        plugin_mcp::mcp_environment::runtime_cwd(server)
    }
    fn http_headers<'a>(&'a self, server: &'a McpServerHttp, _owner: Option<&'a mcp_client::FormOwner>)
        -> Pin<Box<dyn Future<Output = Result<Option<reqwest_012::header::HeaderMap>>> + Send + 'a>> {
        Box::pin(async move { plugin_mcp::mcp_auth::runtime_headers(&Source, server) })
    }
    fn request_elicitation(&self, owner: mcp_client::FormOwner, server: String,
        request: rmcp::model::CreateElicitationRequestParam, cancelled: CancellationToken,
        closed: CancellationToken, wire_schema: Option<serde_json::Value>)
        -> Pin<Box<dyn Future<Output = Result<rmcp::model::CreateElicitationResult, rmcp::model::ErrorData>> + Send + '_>> {
        Box::pin(plugin_mcp::mcp_elicitation::request(owner, server, request, cancelled, closed, wire_schema))
    }
}

async fn with_server_peer<T>(server: &McpServer,
    f: impl for<'a> FnOnce(&'a rmcp::service::Peer<rmcp::RoleClient>)
        -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>,
) -> Result<T> {
    let peer = mcp_client::open_server_peer(Arc::new(Client), server.clone(),
        std::time::Duration::from_secs(20), CancellationToken::new(), None).await?;
    let result = f(peer.peer()).await;
    let _ = peer.cancel().await;
    result
}

mod t27_flow_tests;
mod t28_owner_tests;

// Explicit no-turn transport owner. Session registry, app mounts and dynamic tool
// discovery are deliberately unavailable; their acceptance stays in vw-agent.
struct Connections;
pub(crate) fn connection_host() -> Arc<dyn plugin_mcp::connection_host::ConnectionHost> { Arc::new(Connections) }
impl plugin_mcp::connection_host::ConnectionHost for Connections {
    fn request_abort(&self) -> Result<Option<CancellationToken>> { Ok(None) }
    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> Result<Option<PathBuf>> {
        plugin_mcp::mcp_environment::runtime_cwd(server)
    }
    fn runtime_headers(&self, server: &McpServerHttp, oauth: Option<&str>) -> Result<Option<reqwest_012::header::HeaderMap>> {
        plugin_mcp::mcp_auth::runtime_oauth_headers(&Source, server, oauth)
    }
    fn oauth_access<'a>(&'a self, server: &'a McpServerHttp, owner: &'a mcp_client::FormOwner) -> plugin_mcp::connection_host::HostFuture<'a, Option<String>> {
        Box::pin(plugin_mcp::mcp_oauth_tokens::access(&Source, server, owner))
    }
    fn oauth_cancellation(&self, server: &McpServerHttp, owner: &mcp_client::FormOwner) -> Result<CancellationToken> {
        if plugin_mcp::mcp_auth::source_config(&Source, server)?.is_some() {
            plugin_mcp::mcp_oauth_tokens::connection_cancellation(server, owner)
        } else { Ok(CancellationToken::new()) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn selection_is_current(&self, _server: &McpServer, tenant: Option<&vw_api_transport::TenantScope>) -> Result<bool> {
        anyhow::ensure!(tenant.is_none_or(|tenant| tenant == &Default::default()), "fixture does not own application selections");
        Ok(true)
    }
    fn open_peer(&self, server: McpServer, startup: std::time::Duration, closed: CancellationToken, owner: Option<mcp_client::FormOwner>) -> plugin_mcp::connection_host::HostFuture<'_, mcp_client::RunningMcpPeer> {
        Box::pin(mcp_client::open_server_peer(Arc::new(Client), server, startup, closed, owner))
    }
    fn discover<'a>(&'a self, _server: McpServer, _management: acp_config::McpServerManagement, _existing_ids: &'a [&'a str], _pool: Arc<plugin_mcp::mcp_connections::McpSessionConnections>) -> plugin_mcp::connection_host::HostFuture<'a, Vec<Arc<dyn tool_types::Tool>>> {
        Box::pin(async { anyhow::bail!("dynamic tool discovery requires the real host") })
    }
    fn revoke_app(&self, _tenant: &vw_api_transport::TenantScope, _session: &str, _root: &Path) {
        panic!("application revocation requires the real host");
    }
}

mod t18_tests;
mod t20_tests;
mod t23_tests;

mod tool_host;
mod tools {
 pub use tool_types::{Tool, ToolCallResult, ToolSpec};
 pub mod mcp_tool {
  pub type McpTool = tool_mcp::mcp_tool::McpTool<super::super::tool_host::Host>;
  pub use tool_mcp::mcp_tool::{mcp_tools_to_specs, discover_mcp_tool_specs};
 }
 pub mod mcp_resource { pub type McpResourceTool = tool_mcp::mcp_resource::McpResourceTool<super::super::tool_host::Host>; }
}
mod t19_tests;
mod t21_tests;
mod t22_tests;

mod t24_tests;
