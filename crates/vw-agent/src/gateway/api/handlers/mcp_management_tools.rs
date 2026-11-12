//! Explicit discovery stays under the host's narrow MCP probe authorization.
use super::mcp_management_read::{
    McpHttpError, PolicyFile, access_error, failure, open_database, unavailable,
};
use crate::{
    agent_apps::mcp::network_probe::{McpHandshakeStartError, start_details},
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobError, DiagnosticJobs},
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json,
    extract::{Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
};
use vw_api_types::{
    ProjectScope, TenantScope,
    agent_apps_diagnostics::{DiagnosticJobIdentity, DiagnosticScope, DiagnosticScopeQuery},
    mcp_management::McpToolDetailsRequest,
};

pub(super) async fn submit(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    request: Result<Json<McpToolDetailsRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DiagnosticJobIdentity>), McpHttpError> {
    let Json(request) = request.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    tokio::task::spawn_blocking(move || {
        let scope = DiagnosticScope {
            project: ProjectScope::new(tenant, query.project_id)
                .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
            target_host_id: query.target_host_id,
            app_id: query.app_id,
        };
        let access = super::agent_apps::McpProbeAccess::new(policy.0.clone(), &headers)
            .map_err(access_error)?;
        let grant = access.authorize_details(&scope, &request.record.id).map_err(access_error)?;
        if grant.permit.catalog_scope(&scope) != request.scope
            || grant.permit.connection_revision != request.connection_revision
        {
            return Err(access_error(StatusCode::FORBIDDEN));
        }
        let jobs = jobs.ok_or_else(|| unavailable(()))?.0;
        // The probe validates the scope, membership, trusted host and exact definition before I/O.
        start_details(&jobs, open_database(database)?, policy.0, &headers, scope, request)
            .map(|job| (StatusCode::ACCEPTED, Json(job)))
            .map_err(|error| match error {
                McpHandshakeStartError::Access(status) => access_error(status),
                McpHandshakeStartError::Record(code) => {
                    use vw_api_types::mcp_management::McpFailureCode;
                    let status = match code {
                        McpFailureCode::PermissionDenied => StatusCode::FORBIDDEN,
                        McpFailureCode::NotFound => StatusCode::NOT_FOUND,
                        McpFailureCode::StaleRevision | McpFailureCode::Conflict => {
                            StatusCode::CONFLICT
                        }
                        McpFailureCode::InvalidDefinition => StatusCode::BAD_REQUEST,
                        _ => StatusCode::SERVICE_UNAVAILABLE,
                    };
                    failure(status, code)
                }
                McpHandshakeStartError::Job(error) => access_error(match error {
                    DiagnosticJobError::CapacityExceeded => StatusCode::TOO_MANY_REQUESTS,
                    DiagnosticJobError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                    DiagnosticJobError::NotFound => StatusCode::NOT_FOUND,
                    DiagnosticJobError::InvalidInput => StatusCode::BAD_REQUEST,
                }),
            })
    })
    .await
    .map_err(unavailable)?
}

pub(super) async fn read_acp(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(query): Query<vw_api_types::mcp_management::McpAcpUsageQuery>,
) -> Result<Json<vw_api_types::mcp_management::McpAcpUsage>, McpHttpError> {
    use vw_api_types::mcp_management::{McpAcpUsage, McpAcpUsageMount};
    let project = ProjectScope::new(tenant, query.project_id)
        .map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    tokio::task::spawn_blocking({
        let policy = policy.clone();
        let headers = headers.clone();
        let project = project.clone();
        let host = query.target_host_id.clone();
        let session = query.session_id.clone();
        move || {
            super::agent_apps::authorize_mcp_acp_read(
                policy.0.as_deref(),
                &headers,
                &project,
                &host,
                &session,
            )
        }
    })
    .await
    .map_err(unavailable)?
    .map_err(access_error)?;
    let snapshot = crate::acp::read_catalog_mounts(&project, query.session_id.as_ref())
        .await
        .map_err(|error| {
            access_error(match error {
                vw_acp::server::BackendError::NotFound(_) => StatusCode::NOT_FOUND,
                vw_acp::server::BackendError::OwnershipMismatch(_) => StatusCode::FORBIDDEN,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            })
        })?;
    // Revalidate after storage I/O, before publishing potentially revoked session references.
    tokio::task::spawn_blocking({
        let project = project.clone();
        let host = query.target_host_id.clone();
        let session = query.session_id.clone();
        move || {
            super::agent_apps::authorize_mcp_acp_read(
                policy.0.as_deref(),
                &headers,
                &project,
                &host,
                &session,
            )
        }
    })
    .await
    .map_err(unavailable)?
    .map_err(access_error)?;
    Ok(Json(McpAcpUsage {
        target_host_id: query.target_host_id,
        project: snapshot.project,
        session_id: snapshot.session_id.into(),
        mount_revision: snapshot.revision.0,
        mounts: snapshot
            .catalog
            .into_iter()
            .map(|selection| McpAcpUsageMount {
                scope: selection.mount.scope,
                record: selection.mount.record,
                failure: selection.failure,
            })
            .collect(),
    }))
}
