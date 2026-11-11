//! Explicit MCP connection checks using the host's shared diagnostic contract.
use super::{agent_apps_diagnostics, mcp_management_read::PolicyFile};
use crate::{gateway::agent_apps_diagnostic_jobs::DiagnosticJobs, global_database::GlobalDatabase};
use axum::{
    Extension, Json,
    extract::{Query, rejection::JsonRejection},
    http::HeaderMap,
    response::Response,
};
use serde::Deserialize;
use vw_api_types::{
    TenantScope, agent_apps_diagnostics::DiagnosticScopeQuery, mcp_management::McpRecordId,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ChecksRequest {
    server_ids: Vec<McpRecordId>,
    #[serde(default)]
    generation: u64,
}

pub(super) async fn submit(
    Extension(policy): Extension<PolicyFile>,
    tenant: Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    query: Query<DiagnosticScopeQuery>,
    request: Result<Json<ChecksRequest>, JsonRejection>,
) -> Response {
    // Keep authentication, whole-batch authorization, trusted execution-host
    // resolution, credential lookup and scheduling on the existing diagnostic path.
    agent_apps_diagnostics::submit_request(
        Extension(agent_apps_diagnostics::PolicyFile(policy.0)),
        tenant,
        database,
        jobs,
        headers,
        query,
        request.map(|Json(request)| {
            agent_apps_diagnostics::SubmissionRequest::McpConnectionChecks {
                server_ids: request.server_ids,
                generation: request.generation,
            }
        }),
    )
    .await
}
