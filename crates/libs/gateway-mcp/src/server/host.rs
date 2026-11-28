//! Narrow host actions; authorization and HTTP error mapping stay in this crate.
use std::{future::Future, path::Path};
use vw_api_transport::TenantScope;
use vw_api_mcp_inspect::McpInspectResponse;
pub trait McpHost: Send + Sync + 'static {
    fn inspect(
        workspace: &Path,
        directory: Option<&str>,
        server: Option<&str>,
    ) -> impl Future<Output = anyhow::Result<McpInspectResponse>> + Send;
    fn current_tenant() -> TenantScope;
    fn call(tenant: TenantScope, session: &str, server: &str, binding: &str, name: &str, arguments: serde_json::Value) -> anyhow::Result<serde_json::Value>;
}
