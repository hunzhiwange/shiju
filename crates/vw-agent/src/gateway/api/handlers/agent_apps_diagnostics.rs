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
    extract::{DefaultBodyLimit, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use vw_agent_apps::{auth::copilot::CopilotAuth, diagnostics::endpoints};
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticJobIdentity, DiagnosticProbeKind, DiagnosticScope, DiagnosticScopeQuery,
        DiagnosticStartRequest,
    },
    error::{ApiErrorBody, ApiErrorDetail},
    id::{ProjectScope, TenantScope},
};

#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/diagnostics/jobs", post(start))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(Extension(PolicyFile(policy)))
}

async fn start(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    jobs: Option<Extension<DiagnosticJobs>>,
    headers: HeaderMap,
    Query(query): Query<DiagnosticScopeQuery>,
    request: Result<Json<DiagnosticStartRequest>, JsonRejection>,
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
            return network_probe::start_handshakes(
                &jobs, database, policy.0, &headers, scope, server_ids,
            )
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
