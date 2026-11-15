//! Host actions for the plugin-owned connection lifecycle.
pub use plugin_mcp::mcp_connections::*;
use plugin_mcp::connection_host::{ConnectionHost, HostFuture, MountLease};
use agent_client_protocol::{McpServer, McpServerHttp, McpServerStdio};
use anyhow::Result;
use mcp_client::{FormOwner, RunningMcpPeer};
use std::{sync::Arc, path::{Path, PathBuf}, time::Duration};
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;
use crate::tools::Tool;

pub(crate) fn host() -> Arc<dyn ConnectionHost> { Arc::new(PluginConnectionHost) }
struct PluginConnectionHost;
impl ConnectionHost for PluginConnectionHost {
    fn request_abort(&self) -> Result<Option<CancellationToken>> {
        let context = crate::tools::current_tool_use_context();
        if let Some(snapshot) = context.as_ref().and_then(|context| context.plugin_snapshot()) {
            snapshot.validate_active()?;
        }
        Ok(context.and_then(|c| c.abort_token().cloned()))
    }
    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> Result<Option<PathBuf>> {
        crate::tools::mcp_common::runtime_stdio_cwd(server)
    }
    fn runtime_headers(&self, server: &McpServerHttp, oauth: Option<&str>) -> Result<Option<reqwest_012::header::HeaderMap>> {
        super::mcp_auth::runtime_oauth_headers(server, oauth)
    }
    fn oauth_access<'a>(&'a self, server: &'a McpServerHttp, owner: &'a FormOwner) -> HostFuture<'a, Option<String>> {
        Box::pin(super::mcp_oauth_tokens::access(server, owner))
    }
    fn oauth_cancellation(&self, server: &McpServerHttp, owner: &FormOwner) -> Result<CancellationToken> {
        if super::mcp_auth::source_config(server)?.is_some() {
            super::mcp_oauth_tokens::connection_cancellation(server, owner)
        } else { Ok(CancellationToken::new()) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn selection_is_current(&self, server: &McpServer, tenant: Option<&TenantScope>) -> Result<bool> {
        crate::agent_apps::mcp::revocation::runtime_selection_is_current(server, tenant)
    }
    fn open_peer(&self, server: McpServer, startup: Duration, closed: CancellationToken, owner: Option<FormOwner>) -> HostFuture<'_, RunningMcpPeer> {
        Box::pin(crate::tools::mcp_common::open_server_peer(server, startup, closed, owner))
    }
    fn discover<'a>(&'a self, server: McpServer, management: acp_config::McpServerManagement,
        existing_ids: &'a [&'a str], pool: Arc<McpSessionConnections>) -> HostFuture<'a, Vec<Arc<dyn Tool>>> {
        Box::pin(async move {
            let discovered = crate::tools::discover_dynamic_mcp_tools(server, management, existing_ids, pool).await?;
            Ok(discovered.tools.into_iter().map(|tool| Arc::new(tool) as Arc<dyn Tool>).collect())
        })
    }
    fn revoke_app(&self, tenant: &TenantScope, session: &str, root: &Path) {
        super::mcp_app_execution::revoke_for_tenant(tenant, session, root);
    }
}

struct ManagedMounts(crate::agent_apps::mcp::acp_mount::AcpMountLeases);
impl MountLease for ManagedMounts {
    fn is_current(&self, name: &str) -> Result<bool> { self.0.is_current(name) }
}
pub(crate) fn mount_leases(leases: Option<crate::agent_apps::mcp::acp_mount::AcpMountLeases>) -> Option<Arc<dyn MountLease>> {
    leases.map(|leases| Arc::new(ManagedMounts(leases)) as Arc<dyn MountLease>)
}
