//! Explicit MCP connection checks using the host's shared diagnostic contract.
use axum::http::header;
use super::{host::{ProbeHost, ProbeStartError}, mcp_management_read::PolicyFile};
use agent_database::GlobalDatabase;
use axum::{
    response::IntoResponse,
    http::StatusCode,
    Extension, Json,
    extract::{Query, rejection::JsonRejection},
    http::HeaderMap,
    response::Response,
};
use serde::Deserialize;
use {
    vw_api_transport::TenantScope,
    vw_api_app_diagnostics::DiagnosticScopeQuery,
    vw_api_mcp_management::McpRecordId,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChecksRequest {
    server_ids: Vec<McpRecordId>,
    #[serde(default)]
    generation: u64,
}

pub async fn submit<H: ProbeHost>(
    Extension(policy): Extension<PolicyFile>,
    tenant: Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<H::Jobs>>,
    headers: HeaderMap,
    query: Query<DiagnosticScopeQuery>,
    request: Result<Json<ChecksRequest>, JsonRejection>,
) -> Response {

    use vw_api_transport::ProjectScope;
    use vw_api_app_diagnostics::{DiagnosticScope, DiagnosticStartRequest};
    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return failure(if error.status() == StatusCode::PAYLOAD_TOO_LARGE { StatusCode::PAYLOAD_TOO_LARGE } else { StatusCode::BAD_REQUEST }),
    };
    let result = tokio::task::spawn_blocking(move || {
        let scope = DiagnosticScope {
            project: ProjectScope::new(tenant.0, query.0.project_id).map_err(|_| StatusCode::BAD_REQUEST)?,
            target_host_id: query.0.target_host_id,
            app_id: query.0.app_id,
        };
        super::agent_apps::authorize_diagnostic_probe(policy.0.as_deref(), &headers, &scope,
            &DiagnosticStartRequest::Mcp { server_ids: request.server_ids.clone() })?;
        let jobs = jobs.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        let database = super::mcp_management_read::open_database(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        H::start_checks(&jobs, database, policy.0, &headers, scope, request.server_ids, request.generation)
            .map_err(|error| match error {
                ProbeStartError::Access(status) | ProbeStartError::Job(status) => status,
                ProbeStartError::Record(vw_api_mcp_management::McpFailureCode::PermissionDenied) => StatusCode::FORBIDDEN,
                ProbeStartError::Record(vw_api_mcp_management::McpFailureCode::StorageUnavailable) => StatusCode::SERVICE_UNAVAILABLE,
                ProbeStartError::Record(_) => StatusCode::UNPROCESSABLE_ENTITY,
            })
    }).await.unwrap_or(Err(StatusCode::SERVICE_UNAVAILABLE));
    match result {
        Ok(job) => (StatusCode::ACCEPTED, [(axum::http::header::CACHE_CONTROL, "no-store")], Json(job)).into_response(),
        Err(status) => failure(status),
    }
}

fn failure(status: StatusCode) -> Response {
    let code = match status {
        StatusCode::UNAUTHORIZED => "unauthenticated",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::TOO_MANY_REQUESTS => "capacity_exceeded",
        StatusCode::SERVICE_UNAVAILABLE => "unavailable",
        StatusCode::NOT_IMPLEMENTED => "unsupported_mode",
        _ => "invalid_input",
    };
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(vw_api_transport::error::ApiErrorBody {
            error: vw_api_transport::error::ApiErrorDetail {
                code: code.into(),
                message: "Diagnostic request rejected".into(),
                details: Some(
                    serde_json::json!({"correlationId": uuid::Uuid::new_v4().to_string()}),
                ),
            },
        }),
    )
        .into_response()
}
