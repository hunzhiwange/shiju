use super::mcp_management_read::{PolicyFile, McpHttpError};
use crate::{gateway::agent_apps_diagnostic_jobs::DiagnosticJobs, global_database::GlobalDatabase};
use axum::{Extension, Json, extract::{Query, rejection::JsonRejection}, http::{HeaderMap, StatusCode}};
use vw_api_transport::TenantScope;
use vw_api_app_diagnostics::{DiagnosticScopeQuery, DiagnosticJobIdentity};
use vw_api_mcp_management::McpToolDetailsRequest;
pub(super) async fn submit(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    request: Result<Json<McpToolDetailsRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DiagnosticJobIdentity>), McpHttpError> {
    gateway_mcp::server::mcp_management_tools::submit::<super::mcp_host::Host>(Extension(policy), Extension(tenant), database, jobs, headers, Query(query), request).await
}
pub(super) async fn read_acp(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(query): Query<vw_api_mcp_management::McpAcpUsageQuery>,
) -> Result<Json<vw_api_mcp_management::McpAcpUsage>, McpHttpError> {
    gateway_mcp::server::mcp_management_tools::read_acp::<super::mcp_host::Host>(Extension(policy), Extension(tenant), headers, Query(query)).await
}
