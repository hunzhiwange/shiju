//! Existing application actions required by connection admission and discovery.
use std::{sync::Arc, path::{Path, PathBuf}, time::Duration, future::Future, pin::Pin};
use agent_client_protocol::{McpServer, McpServerHttp, McpServerStdio};
use anyhow::Result;
use mcp_client::{RunningMcpPeer, FormOwner};
use tokio_util::sync::CancellationToken;
use vw_api_transport::TenantScope;
use tool_types::Tool;
use super::mcp_connections::McpSessionConnections;

pub type HostFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub trait MountLease: Send + Sync {
    fn is_current(&self, name: &str) -> Result<bool>;
}

pub trait ConnectionHost: Send + Sync {
    /// Validate the active plugin snapshot before returning the current request cancellation.
    fn request_abort(&self) -> Result<Option<CancellationToken>>;
    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> Result<Option<PathBuf>>;
    fn runtime_headers(&self, server: &McpServerHttp, oauth: Option<&str>) -> Result<Option<reqwest_012::header::HeaderMap>>;
    fn oauth_access<'a>(&'a self, server: &'a McpServerHttp, owner: &'a FormOwner) -> HostFuture<'a, Option<String>>;
    /// Revalidate source provenance before returning its connection cancellation token.
    fn oauth_cancellation(&self, server: &McpServerHttp, owner: &FormOwner) -> Result<CancellationToken>;
    #[cfg(not(target_arch = "wasm32"))]
    fn selection_is_current(&self, server: &McpServer, tenant: Option<&TenantScope>) -> Result<bool>;
    fn open_peer(&self, server: McpServer, startup: Duration, closed: CancellationToken, owner: Option<FormOwner>) -> HostFuture<'_, RunningMcpPeer>;
    fn discover<'a>(&'a self, server: McpServer, management: acp_config::McpServerManagement,
        existing_ids: &'a [&'a str], pool: Arc<McpSessionConnections>) -> HostFuture<'a, Vec<Arc<dyn Tool>>>;
    fn revoke_app(&self, tenant: &TenantScope, session: &str, root: &Path);
}
