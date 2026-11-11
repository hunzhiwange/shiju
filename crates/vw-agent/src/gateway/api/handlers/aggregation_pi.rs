//! Project-authorized Pi directory views and revision-checked single-entry writes.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::put,
};
use std::path::PathBuf;
use vw_api_types::{agent_apps::aggregation::*, id::TenantScope};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
type DirectoryResult = Result<PiDirectoryState, (StatusCode, &'static str)>;
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/pi/{id}/directory-entry", put(write))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
pub(super) async fn read(
    policy: Option<PathBuf>,
    tenant: TenantScope,
    headers: HeaderMap,
    query: AggregationQuery,
) -> Response {
    respond(
        tokio::task::spawn_blocking(move || execute(policy, tenant, headers, query, None)).await,
    )
}
async fn write(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    id: Result<Path<String>, axum::extract::rejection::PathRejection>,
    body: Result<Json<PiDirectoryWrite>, JsonRejection>,
) -> Response {
    let Ok(Path(id)) = id else { return invalid_request() };
    let Ok(Json(body)) = body else { return error(StatusCode::BAD_REQUEST, "invalid_request") };
    let query = AggregationQuery {
        target_host_id: body.target_host_id.clone(),
        project_id: body.project_id.clone(),
    };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy.0, tenant, headers, query, Some((id, body)))
        })
        .await,
    )
}
pub(super) fn invalid_request() -> Response {
    error(StatusCode::BAD_REQUEST, "invalid_request")
}
fn error(status: StatusCode, code: &str) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(ApiErrorBody {
            error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
        }),
    )
        .into_response()
}
fn respond(result: Result<DirectoryResult, tokio::task::JoinError>) -> Response {
    match result {
        Ok(Ok(state)) => ([(header::CACHE_CONTROL, "no-store")], Json(state)).into_response(),
        Ok(Err((status, code))) => error(status, code),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"),
    }
}
#[cfg(target_os = "macos")]
fn status_error(status: StatusCode) -> (StatusCode, &'static str) {
    (
        status,
        match status {
            StatusCode::UNAUTHORIZED => "unauthenticated",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::BAD_REQUEST => "invalid_request",
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::UNPROCESSABLE_ENTITY => "unsupported_target",
            _ => "target_unavailable",
        },
    )
}
#[cfg(not(target_os = "macos"))]
fn execute(
    _: Option<PathBuf>,
    _: TenantScope,
    _: HeaderMap,
    _: AggregationQuery,
    _: Option<(String, PiDirectoryWrite)>,
) -> DirectoryResult {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Option<PathBuf>,
    tenant: TenantScope,
    headers: HeaderMap,
    query: AggregationQuery,
    write: Option<(String, PiDirectoryWrite)>,
) -> DirectoryResult {
    use vw_agent_apps::providers::{
        live::{
            commit::{ExecutionHostId, ProviderTransactions},
            write_guard::lock_app,
        },
        pi::{self, PiDirectoryWriteError, PiFiles, PiNativeDirectory},
    };
    use vw_api_types::{agent_apps::AppId, id::ProjectScope};
    let project = ProjectScope::new(tenant, query.project_id)
        .map_err(|_| status_error(StatusCode::BAD_REQUEST))?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id)
        .map_err(|_| status_error(StatusCode::BAD_REQUEST))?;
    let context = super::agent_apps::authorize_pi_directory(
        policy.as_deref(),
        &headers,
        &project,
        &query.target_host_id,
        write.is_some(),
    )
    .map_err(status_error)?;
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable");
    let conflict = || (StatusCode::CONFLICT, "revision_conflict");
    let root = context.credential_root.as_deref().ok_or_else(unavailable)?;
    // This lock does not create files; reads cannot claim an unowned native slot.
    let _guard = lock_app(AppId::Pi, &context.home).map_err(|_| unavailable())?;
    let tx = ProviderTransactions::open_existing(
        root,
        ExecutionHostId::new(&query.target_host_id)
            .map_err(|_| status_error(StatusCode::BAD_REQUEST))?,
    )
    .map_err(|_| unavailable())?
    .ok_or_else(conflict)?;
    let state = tx.state(AppId::Pi).map_err(|_| unavailable())?;
    if state.project_scope.as_ref() != Some(&project) {
        return Err(if state.project_scope.is_some() {
            status_error(StatusCode::FORBIDDEN)
        } else {
            conflict()
        });
    }
    state.require_project(&project).map_err(|_| conflict())?;
    if tx.pending(AppId::Pi).map_err(|_| unavailable())?.is_some() {
        return Err((StatusCode::CONFLICT, "recovery_pending"));
    }
    let env = std::env::var_os("PI_CODING_AGENT_DIR");
    let capture = || {
        pi::read_native_directory(&context.home, context.directory.as_deref(), env.as_deref())
            .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"))
    };
    let revision = |native: &PiNativeDirectory| {
        super::aggregation_read_snapshot::sign(
            &serde_json::to_vec(&(
                "pi_directory_revision_v1",
                &project,
                &query.target_host_id,
                &context.principal,
                &state,
                &native.source_revision,
                &native.default_provider_id,
                &native.default_model_id,
            ))
            .map_err(|_| unavailable())?,
        )
        .map_err(status_error)
    };
    let mut native = capture()?;
    let current_revision = revision(&native)?;
    let mut write_files;
    if let Some((id, body)) = write {
        vw_agent_apps::providers::validate_identity(&id)
            .map_err(|_| status_error(StatusCode::BAD_REQUEST))?;
        if body.expected_revision.is_empty() || body.expected_revision != current_revision {
            return Err(conflict());
        }
        let change = match &body.change {
            PiDirectoryChange::Insert { profile } => pi::PiDirectoryChange::Insert(profile),
            PiDirectoryChange::Replace { profile } => pi::PiDirectoryChange::Replace(profile),
            PiDirectoryChange::Remove {} => pi::PiDirectoryChange::Remove,
        };
        drop(_guard);
        write_files =
            PiFiles::open_directory(&context.home, context.directory.as_deref(), env.as_deref())
                .map_err(|error| match error {
                    vw_agent_apps::providers::live::write_guard::WriteError::InvalidResource => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration")
                    }
                    _ => (StatusCode::BAD_GATEWAY, "target_failed"),
                })?;
        let unchanged = || {
            tx.state(AppId::Pi).ok().as_ref() == Some(&state)
                && capture().and_then(|n| revision(&n)).is_ok_and(|r| r == current_revision)
        };
        write_files
            .apply_directory_change_for_project(
                &tx,
                &project,
                &native.source_revision,
                &id,
                change,
                unchanged,
            )
            .map_err(|error| match error {
                PiDirectoryWriteError::Forbidden => status_error(StatusCode::FORBIDDEN),
                PiDirectoryWriteError::InvalidConfiguration => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration")
                }
                PiDirectoryWriteError::AlreadyExists => (StatusCode::CONFLICT, "already_exists"),
                PiDirectoryWriteError::NotFound => status_error(StatusCode::NOT_FOUND),
                PiDirectoryWriteError::RecoveryPending => {
                    (StatusCode::CONFLICT, "recovery_pending")
                }
                PiDirectoryWriteError::RevisionConflict => conflict(),
                PiDirectoryWriteError::TargetFailed => (StatusCode::BAD_GATEWAY, "target_failed"),
            })?;
        native = capture()?;
    } else if tx.state(AppId::Pi).map_err(|_| unavailable())? != state
        || tx.pending(AppId::Pi).map_err(|_| unavailable())?.is_some()
        || revision(&capture()?)? != current_revision
    {
        return Err(conflict());
    }
    // Keep the native lock until the response revision reflects the committed state.
    let after = tx.state(AppId::Pi).map_err(|_| unavailable())?;
    let revision = super::aggregation_read_snapshot::sign(
        &serde_json::to_vec(&(
            "pi_directory_revision_v1",
            &project,
            &query.target_host_id,
            &context.principal,
            &after,
            &native.source_revision,
            &native.default_provider_id,
            &native.default_model_id,
        ))
        .map_err(|_| unavailable())?,
    )
    .map_err(status_error)?;
    Ok(PiDirectoryState {
        revision,
        members: native.members,
        default_provider_id: native.default_provider_id,
        default_model_id: native.default_model_id,
    })
}
