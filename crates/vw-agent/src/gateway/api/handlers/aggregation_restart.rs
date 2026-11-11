//! Confirmed, independently authorized restart through the provider task registry.
use super::app_providers::operations::{State, aggregation::Owner};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::{ProjectScope, TenantScope},
};

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/{app}/aggregation/codex/restart", post(submit))
        .layer(DefaultBodyLimit::max(4096))
}
#[derive(Debug)]
struct Failure(StatusCode, &'static str);
impl From<StatusCode> for Failure {
    fn from(status: StatusCode) -> Self {
        Self(
            status,
            match status {
                StatusCode::BAD_REQUEST => "invalid_request",
                StatusCode::UNAUTHORIZED => "unauthenticated",
                StatusCode::FORBIDDEN => "forbidden",
                StatusCode::NOT_FOUND => "not_found",
                StatusCode::CONFLICT => "revision_conflict",
                StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_IMPLEMENTED => {
                    "unsupported_target"
                }
                StatusCode::BAD_GATEWAY => "target_failed",
                _ => "target_unavailable",
            },
        )
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
        (
            self.0,
            [(header::CACHE_CONTROL, "no-store")],
            Json(ApiErrorBody {
                error: ApiErrorDetail {
                    code: self.1.into(),
                    message: self.1.into(),
                    details: None,
                },
            }),
        )
            .into_response()
    }
}
async fn submit(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    input: Result<Json<CodexRestartRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let (Ok(Path(app)), Ok(Json(request))) = (app, input) else {
        return Failure::from(StatusCode::BAD_REQUEST).into_response();
    };
    match tokio::task::spawn_blocking(move || prepare(state, tenant, headers, app, request)).await {
        Ok(Ok(progress)) => {
            (StatusCode::ACCEPTED, [(header::CACHE_CONTROL, "no-store")], Json(progress))
                .into_response()
        }
        Ok(Err(error)) => error.into_response(),
        Err(_) => Failure::from(StatusCode::BAD_GATEWAY).into_response(),
    }
}
#[cfg(not(target_os = "macos"))]
fn prepare(
    _: State,
    _: TenantScope,
    _: HeaderMap,
    _: AppId,
    _: CodexRestartRequest,
) -> Result<serde_json::Value, Failure> {
    Err(StatusCode::UNPROCESSABLE_ENTITY.into())
}
#[cfg(target_os = "macos")]
fn restart_error(error: vw_agent_apps::aggregation::daemon_restart::RestartError) -> Failure {
    use vw_agent_apps::aggregation::daemon_restart::RestartError::*;
    match error {
        ConfirmationRequired => Failure(StatusCode::UNPROCESSABLE_ENTITY, "confirmation_required"),
        Forbidden => StatusCode::FORBIDDEN.into(),
        RevisionConflict => StatusCode::CONFLICT.into(),
        UnsupportedTarget => StatusCode::UNPROCESSABLE_ENTITY.into(),
        TargetUnavailable => StatusCode::SERVICE_UNAVAILABLE.into(),
        TargetFailed => StatusCode::BAD_GATEWAY.into(),
        TargetTimeout => Failure(StatusCode::GATEWAY_TIMEOUT, "target_timeout"),
    }
}
#[cfg(target_os = "macos")]
fn prepare(
    state: State,
    tenant: TenantScope,
    headers: HeaderMap,
    app: AppId,
    request: CodexRestartRequest,
) -> Result<serde_json::Value, Failure> {
    use vw_agent_apps::{
        aggregation::{catalog_generation, daemon_restart},
        providers::{
            codex_catalog::CodexFiles,
            live::{
                commit::{ExecutionHostId, ProviderTransactions},
                write_guard::lock_app,
            },
        },
    };
    let project = ProjectScope::new(tenant, request.project_id.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    for id in [&request.target_host_id, &request.request_id] {
        vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    if request.expected_revision.trim().is_empty() || request.expected_revision.len() > 256 {
        return Err(StatusCode::BAD_REQUEST.into());
    }
    let policy = state.aggregation_policy().map(std::path::Path::to_path_buf);
    let authorize = || {
        super::agent_apps::authorize_codex_restart(
            policy.as_deref(),
            &headers,
            &project,
            &request.target_host_id,
        )
    };
    let (env, root, principal) = authorize()?;
    if app != AppId::Codex {
        return Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"));
    }
    let owner = Owner {
        project: project.clone(),
        host: request.target_host_id.clone(),
        app,
        principal: principal.clone(),
    };
    // An action domain prevents reuse of a publication request ID from authorizing restart.
    let fingerprint: [u8; 32] = Sha256::digest(
        [
            b"codex_restart:".as_slice(),
            &serde_json::to_vec(&request).map_err(|_| StatusCode::BAD_REQUEST)?,
        ]
        .concat(),
    )
    .into();
    let replay = || {
        state.aggregation_restart_replay(&owner, &request.request_id, &fingerprint).map_err(|s| {
            if s == StatusCode::CONFLICT { Failure(s, "request_conflict") } else { s.into() }
        })
    };
    if let Some(progress) = replay()? {
        return Ok(progress);
    }
    if !request.confirmed {
        return Err(Failure(StatusCode::UNPROCESSABLE_ENTITY, "confirmation_required"));
    }
    let root = root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let executable = vw_agent_apps::apps::discovery::read::locate_cli(app, &env)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let home = env.home;
    let tx = ProviderTransactions::open_existing(
        &root,
        ExecutionHostId::new(&request.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .ok_or(StatusCode::CONFLICT)?;
    let files = CodexFiles {
        guard: lock_app(app, &home.join(".codex")).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    if tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.project_scope.as_ref()
        != Some(&project)
    {
        return Err(StatusCode::FORBIDDEN.into());
    }
    if tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some() {
        return Err(Failure(StatusCode::CONFLICT, "recovery_pending"));
    }
    if catalog_generation::read(&files, &tx, &project)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .revision()
        != request.expected_revision
    {
        return Err(StatusCode::CONFLICT.into());
    }
    drop(files);
    state
        .aggregation_restart_start(owner, request.request_id.clone(), fingerprint, move || {
            let result = (|| -> Result<CodexRestartResult, Failure> {
                // The worker reloads host policy, physical ownership and revision before handoff.
                let valid = || {
                    super::agent_apps::authorize_codex_restart(
                        policy.as_deref(),
                        &headers,
                        &project,
                        &request.target_host_id,
                    )
                    .is_ok_and(|(env, r, p)| {
                        p == principal
                            && r.as_ref() == Some(&root)
                            && env.home == home
                            && vw_agent_apps::apps::discovery::read::locate_cli(app, &env)
                                .is_ok_and(|path| path == executable)
                    })
                };
                if !valid() {
                    return Err(StatusCode::FORBIDDEN.into());
                }
                let files = CodexFiles {
                    guard: lock_app(app, &home.join(".codex"))
                        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
                };
                if tx
                    .state(app)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                    .project_scope
                    .as_ref()
                    != Some(&project)
                {
                    return Err(StatusCode::FORBIDDEN.into());
                }
                if tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some() {
                    return Err(Failure(StatusCode::CONFLICT, "recovery_pending"));
                }
                vw_agent_apps::providers::operations::publication()
                    .map_err(|_| StatusCode::CONFLICT)?;
                let mut operation = daemon_restart::restart_daemon(
                    &files,
                    &tx,
                    &project,
                    daemon_restart::RestartRequest {
                        expected_revision: &request.expected_revision,
                        confirmed: request.confirmed,
                        executable: &executable,
                    },
                    valid,
                )
                .map_err(restart_error)?;
                drop(files);
                // Always retain the native operation until its real terminal result, even on DELETE.
                match tokio::runtime::Handle::current()
                    .block_on(operation.wait())
                    .map_err(restart_error)?
                {
                    daemon_restart::RestartOutcome::Restarted => Ok(CodexRestartResult::Restarted),
                    daemon_restart::RestartOutcome::NotRunning => {
                        Ok(CodexRestartResult::NotRunning)
                    }
                }
            })();
            match result {
                Ok(result) => AggregationOutcome::Restarted { result },
                Err(e) => AggregationOutcome::Failed {
                    status: e.0.as_u16(),
                    code: serde_json::from_value(serde_json::json!(e.1))
                        .unwrap_or(AggregationErrorCode::TargetFailed),
                    recovery: RecoveryState::None,
                },
            }
        })
        .map_err(
            |s| if s == StatusCode::CONFLICT { Failure(s, "request_conflict") } else { s.into() },
        )
}
