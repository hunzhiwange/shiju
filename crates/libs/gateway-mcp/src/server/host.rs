//! Narrow host actions; authorization and HTTP error mapping stay in this crate.
use std::{future::Future, path::Path};
use vw_api_transport::TenantScope;
use vw_api_mcp_inspect::McpInspectResponse;
pub trait McpHost: Send + Sync + 'static {
    fn inspect(workspace: &Path) -> impl Future<Output = anyhow::Result<McpInspectResponse>> + Send;
    fn current_tenant() -> TenantScope;
    fn call(tenant: TenantScope, session: &str, server: &str, binding: &str, name: &str, arguments: serde_json::Value) -> anyhow::Result<serde_json::Value>;
}

#[cfg(target_os = "macos")]
pub trait ProbeHost: Send + Sync + 'static {
    type Jobs: Clone + Send + Sync + 'static;
    fn start_details(jobs: &Self::Jobs, database: agent_database::GlobalDatabase, policy: Option<std::path::PathBuf>, headers: &axum::http::HeaderMap, scope: vw_api_app_diagnostics::DiagnosticScope, request: vw_api_mcp_management::McpToolDetailsRequest) -> Result<vw_api_app_diagnostics::DiagnosticJobIdentity, ProbeStartError>;
    fn start_checks(jobs: &Self::Jobs, database: agent_database::GlobalDatabase, policy: Option<std::path::PathBuf>, headers: &axum::http::HeaderMap, scope: vw_api_app_diagnostics::DiagnosticScope, ids: Vec<vw_api_mcp_management::McpRecordId>, generation: u64) -> Result<vw_api_app_diagnostics::DiagnosticJobIdentity, ProbeStartError>;
    fn read_mounts(target_host_id: String, project: &vw_api_transport::ProjectScope, session: &str) -> impl Future<Output = Result<vw_api_mcp_management::McpAcpUsage, axum::http::StatusCode>> + Send;
}
#[cfg(target_os = "macos")]
pub enum ProbeStartError {
    Access(axum::http::StatusCode),
    Record(vw_api_mcp_management::McpFailureCode),
    Job(axum::http::StatusCode),
}
