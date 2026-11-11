//! Start diagnostics from authorized saved resources on their execution host.
use crate::{
    agent_apps::{
        auth_accounts::AuthAccountStore,
        mcp::network_probe::{self, McpHandshakeStartError},
    },
    app_providers::AppProviderStore,
    gateway::{
        agent_apps_diagnostic_executors::{
            model_check::{ModelPrepareError, ModelProbe},
            reachability::{ReachabilityPrepareError, ReachabilityProbe},
            speedtest,
            stream_check::StreamProbe,
        },
        agent_apps_diagnostic_jobs::{DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs},
    },
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use vw_agent_apps::{auth::copilot::CopilotAuth, diagnostics::endpoints};
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticEndpointCandidate, DiagnosticEndpointCandidatesQuery,
        DiagnosticEndpointCandidatesResponse, DiagnosticEndpointId, DiagnosticJobIdentity,
        DiagnosticProbeKind, DiagnosticScope, DiagnosticScopeQuery, DiagnosticStartRequest,
    },
    error::{ApiErrorBody, ApiErrorDetail},
    id::{ProjectScope, TaskId, TenantScope},
};

#[derive(Clone)]
pub(super) struct PolicyFile(pub(super) Option<PathBuf>);

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/diagnostics/endpoint-candidates/query", post(endpoint_candidates))
        .route("/agent-apps/diagnostics/jobs", post(submit))
        .route("/agent-apps/diagnostics/jobs/{id}", get(read_job))
        .route("/agent-apps/diagnostics/jobs/{id}/cancel", post(cancel_job))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(PolicyFile(policy)))
}

/// Read authorized editor candidates without resolving credentials or starting work.
async fn endpoint_candidates(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    request: Result<Json<DiagnosticEndpointCandidatesQuery>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => {
            return failure(if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            });
        }
    };
    let result = tokio::task::spawn_blocking(move || {
        let scope = DiagnosticScope {
            project: ProjectScope::new(tenant, query.project_id)
                .map_err(|_| StatusCode::BAD_REQUEST)?,
            target_host_id: query.target_host_id,
            app_id: query.app_id,
        };
        super::agent_apps::authorize_diagnostic_probe(
            policy.0.as_deref(),
            &headers,
            &scope,
            &DiagnosticStartRequest::Speed {
                provider_id: request.provider_id.clone(),
                endpoint_ids: Vec::new(),
            },
        )?;
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let stored = AppProviderStore::preview_record(
            &database,
            scope.project.tenant_scope(),
            scope.app_id,
            request.provider_id.as_ref(),
        )
        .map_err(|error| match error.as_str() {
            "application provider storage unavailable" => StatusCode::SERVICE_UNAVAILABLE,
            "application provider not found" => StatusCode::NOT_FOUND,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        })?;
        let candidates = endpoints::collect(
            scope.app_id,
            &request.draft_urls,
            &stored,
            request.preset_name.as_deref(),
        )
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
        .into_iter()
        .map(|candidate| DiagnosticEndpointCandidate {
            id: DiagnosticEndpointId(hex::encode(Sha256::digest(candidate.url.as_bytes()))),
            display_url: candidate.url,
            sources: candidate.sources,
        })
        .collect();
        Ok(DiagnosticEndpointCandidatesResponse { candidates })
    })
    .await
    .unwrap_or(Err(StatusCode::SERVICE_UNAVAILABLE));
    match result {
        Ok(response) => {
            (StatusCode::OK, [(header::CACHE_CONTROL, "no-store")], Json(response)).into_response()
        }
        Err(status) => failure(status),
    }
}

async fn read_job(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    Path(id): Path<TaskId>,
) -> Response {
    job_request(policy, tenant, jobs, headers, query, id, false).await
}

async fn cancel_job(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    Path(id): Path<TaskId>,
) -> Response {
    job_request(policy, tenant, jobs, headers, query, id, true).await
}

async fn job_request(
    policy: PolicyFile,
    tenant: TenantScope,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    query: DiagnosticScopeQuery,
    id: TaskId,
    cancel: bool,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        let scope = DiagnosticScope {
            project: ProjectScope::new(tenant, query.project_id)
                .map_err(|_| StatusCode::BAD_REQUEST)?,
            target_host_id: query.target_host_id,
            app_id: query.app_id,
        };
        let principal =
            super::agent_apps::authorize_diagnostic_job(policy.0.as_deref(), &headers, &scope)?;
        let owner = DiagnosticJobOwner { scope, principal };
        let jobs = jobs.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        if cancel { jobs.cancel(&owner, &id) } else { jobs.snapshot(&owner, &id) }
            .map_err(job_status)
    })
    .await
    .unwrap_or(Err(StatusCode::SERVICE_UNAVAILABLE));
    match result {
        Ok(snapshot) => {
            (StatusCode::OK, [(header::CACHE_CONTROL, "no-store")], Json(snapshot)).into_response()
        }
        Err(status) => failure(status),
    }
}

/// Existing diagnostics keep their original reports and request contract.
pub(super) async fn submit(
    policy: Extension<PolicyFile>,
    tenant: Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    query: Query<DiagnosticScopeQuery>,
    request: Result<Json<DiagnosticStartRequest>, JsonRejection>,
) -> Response {
    submit_request(
        policy,
        tenant,
        database,
        jobs,
        headers,
        query,
        request.map(|Json(request)| SubmissionRequest::Diagnostic(request)),
    )
    .await
}

pub(super) enum SubmissionRequest {
    Diagnostic(DiagnosticStartRequest),
    McpConnectionChecks {
        server_ids: Vec<vw_api_types::mcp_management::McpRecordId>,
        generation: u64,
    },
}

/// Both HTTP resources pass through the same whole-batch authorization and host resolution.
pub(super) async fn submit_request(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    request: Result<SubmissionRequest, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(request) => request,
        Err(error) => {
            return failure(if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            });
        }
    };
    let (request, generation) = match request {
        SubmissionRequest::Diagnostic(request) => (request, None),
        SubmissionRequest::McpConnectionChecks { server_ids, generation } => {
            (DiagnosticStartRequest::Mcp { server_ids }, Some(generation))
        }
    };
    let result = tokio::task::spawn_blocking(move || {
        let scope = DiagnosticScope {
            project: ProjectScope::new(tenant, query.project_id)
                .map_err(|_| StatusCode::BAD_REQUEST)?,
            target_host_id: query.target_host_id,
            app_id: query.app_id,
        };
        // Authenticate and authorize before resolving storage, credentials, or work.
        let (credential_root, principal) = super::agent_apps::authorize_diagnostic_probe(
            policy.0.as_deref(),
            &headers,
            &scope,
            &request,
        )?;
        let jobs = jobs.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        if let DiagnosticStartRequest::Mcp { server_ids } = request {
            return match generation {
                Some(generation) => network_probe::start_connection_checks(
                    &jobs, database, policy.0, &headers, scope, server_ids, generation,
                ),
                None => network_probe::start_handshakes(
                    &jobs, database, policy.0, &headers, scope, server_ids,
                ),
            }
            .map_err(|error| match error {
                McpHandshakeStartError::Access(status) => status,
                McpHandshakeStartError::Job(error) => job_status(error),
                McpHandshakeStartError::Record(
                    vw_api_types::mcp_management::McpFailureCode::PermissionDenied,
                ) => StatusCode::FORBIDDEN,
                McpHandshakeStartError::Record(
                    vw_api_types::mcp_management::McpFailureCode::StorageUnavailable,
                ) => StatusCode::SERVICE_UNAVAILABLE,
                McpHandshakeStartError::Record(_) => StatusCode::UNPROCESSABLE_ENTITY,
            });
        }
        let owner = DiagnosticJobOwner { scope, principal };
        match request {
            DiagnosticStartRequest::Speed { provider_id, endpoint_ids } => {
                if endpoint_ids.is_empty()
                    || endpoint_ids.len()
                        > crate::gateway::agent_apps_diagnostic_jobs::MAX_PROBES_PER_JOB
                {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let row = AppProviderStore::preview_record(
                    &database,
                    owner.scope.project.tenant_scope(),
                    owner.scope.app_id,
                    provider_id.as_ref(),
                )
                .map_err(|error| match error.as_str() {
                    "application provider storage unavailable" => StatusCode::SERVICE_UNAVAILABLE,
                    "application provider not found" => StatusCode::NOT_FOUND,
                    _ => StatusCode::UNPROCESSABLE_ENTITY,
                })?;
                let candidates = endpoints::collect(owner.scope.app_id, &[], &row, None)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let mut candidates = candidates
                    .into_iter()
                    .map(|candidate| {
                        (hex::encode(Sha256::digest(candidate.url.as_bytes())), candidate)
                    })
                    .collect::<std::collections::HashMap<_, _>>();
                let targets = endpoint_ids
                    .into_iter()
                    .map(|id| {
                        let candidate = candidates.remove(&id.0).ok_or(StatusCode::BAD_REQUEST)?;
                        Ok((id, candidate))
                    })
                    .collect::<Result<Vec<_>, StatusCode>>()?;
                speedtest::start(&jobs, owner, targets, None).map_err(job_status)
            }
            request => {
                let root = credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
                let accounts = AuthAccountStore::new(database.clone(), &root)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                let copilot = CopilotAuth::new().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                match request {
                    DiagnosticStartRequest::Reachability { provider_id } => {
                        let probe = ReachabilityProbe::prepare(
                            &database,
                            &accounts,
                            owner.clone(),
                            provider_id,
                        )
                        .map_err(|error| match error {
                            ReachabilityPrepareError::NotFound => StatusCode::NOT_FOUND,
                            ReachabilityPrepareError::Unavailable => {
                                StatusCode::SERVICE_UNAVAILABLE
                            }
                            _ => StatusCode::UNPROCESSABLE_ENTITY,
                        })?;
                        jobs.start(
                            owner,
                            DiagnosticProbeKind::Reachability,
                            vec![move |cancel| async move {
                                probe.run(&accounts, &copilot, cancel).await
                            }],
                        )
                        .map_err(job_status)
                    }
                    DiagnosticStartRequest::Model { provider_id, model_id } => {
                        let probe = ModelProbe::prepare(
                            &database,
                            &accounts,
                            owner.clone(),
                            provider_id,
                            model_id,
                        )
                        .map_err(model_status)?;
                        jobs.start(
                            owner,
                            DiagnosticProbeKind::Model,
                            vec![move |cancel| async move {
                                probe.run(&accounts, &copilot, cancel).await
                            }],
                        )
                        .map_err(job_status)
                    }
                    DiagnosticStartRequest::Streaming { provider_id, model_id } => {
                        let probe = StreamProbe::prepare(
                            &database,
                            &accounts,
                            owner.clone(),
                            provider_id,
                            model_id,
                        )
                        .map_err(model_status)?;
                        jobs.start(
                            owner,
                            DiagnosticProbeKind::Streaming,
                            vec![move |cancel| async move {
                                probe.run(&accounts, &copilot, cancel).await
                            }],
                        )
                        .map_err(job_status)
                    }
                    // Both variants are handled before constructing credential services.
                    DiagnosticStartRequest::Mcp { .. } | DiagnosticStartRequest::Speed { .. } => {
                        Err(StatusCode::BAD_REQUEST)
                    }
                }
            }
        }
    })
    .await
    .unwrap_or(Err(StatusCode::SERVICE_UNAVAILABLE));
    match result {
        Ok(job) => accepted(job),
        Err(status) => failure(status),
    }
}

fn accepted(job: DiagnosticJobIdentity) -> Response {
    (StatusCode::ACCEPTED, [(header::CACHE_CONTROL, "no-store")], Json(job)).into_response()
}

fn model_status(error: ModelPrepareError) -> StatusCode {
    match error {
        ModelPrepareError::NotFound => StatusCode::NOT_FOUND,
        ModelPrepareError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

fn job_status(error: DiagnosticJobError) -> StatusCode {
    match error {
        DiagnosticJobError::InvalidInput => StatusCode::BAD_REQUEST,
        DiagnosticJobError::CapacityExceeded => StatusCode::TOO_MANY_REQUESTS,
        DiagnosticJobError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        DiagnosticJobError::NotFound => StatusCode::NOT_FOUND,
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
        Json(ApiErrorBody {
            error: ApiErrorDetail {
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
