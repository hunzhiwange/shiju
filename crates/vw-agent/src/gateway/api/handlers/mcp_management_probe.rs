use super::mcp_management_read::PolicyFile;
use crate::{gateway::agent_apps_diagnostic_jobs::DiagnosticJobs, global_database::GlobalDatabase};
use axum::{Extension, Json, extract::{Query, rejection::JsonRejection}, http::HeaderMap, response::Response};
use vw_api_transport::TenantScope;
use vw_api_app_diagnostics::DiagnosticScopeQuery;
pub(crate) use gateway_mcp::server::mcp_management_probe::ChecksRequest;
pub(super) async fn submit(
    Extension(policy): Extension<PolicyFile>,
    tenant: Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    query: Query<DiagnosticScopeQuery>,
    request: Result<Json<ChecksRequest>, JsonRejection>,
) -> Response {
    gateway_mcp::server::mcp_management_probe::submit::<super::mcp_host::Host>(Extension(policy), tenant, database, jobs, headers, query, request).await
}
