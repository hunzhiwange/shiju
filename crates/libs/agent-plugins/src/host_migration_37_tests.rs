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
