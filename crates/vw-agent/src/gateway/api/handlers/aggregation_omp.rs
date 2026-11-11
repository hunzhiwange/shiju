//! Project-scoped OMP native directory API. MCP and login use their existing owners.
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
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/omp/{id}/directory-entry", put(write))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(path)))
}
// State extends the existing OMP route; there is exactly one route owner.
pub(super) async fn read_scoped(
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
    Path(id): Path<String>,
    request: Result<Json<OmpDirectoryWrite>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = request else {
        return failure(StatusCode::BAD_REQUEST, "invalid_input");
    };
    let query = AggregationQuery {
        target_host_id: request.target_host_id.clone(),
        project_id: request.project_id.clone(),
    };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy.0, tenant, headers, query, Some((id, request)))
        })
        .await,
    )
}
type Error = (StatusCode, &'static str);
fn respond(result: Result<Result<OmpDirectoryState, Error>, tokio::task::JoinError>) -> Response {
    match result {
        Ok(Ok(state)) => ([(header::CACHE_CONTROL, "no-store")], Json(state)).into_response(),
        Ok(Err((status, code))) => failure(status, code),
        Err(_) => failure(StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"),
    }
}
pub(super) fn failure(status: StatusCode, code: &str) -> Response {
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
#[cfg(not(target_os = "macos"))]
fn execute(
    _: Option<PathBuf>,
    _: TenantScope,
    _: HeaderMap,
    _: AggregationQuery,
    _: Option<(String, OmpDirectoryWrite)>,
) -> Result<OmpDirectoryState, Error> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Option<PathBuf>,
    tenant: TenantScope,
    headers: HeaderMap,
    query: AggregationQuery,
    request: Option<(String, OmpDirectoryWrite)>,
) -> Result<OmpDirectoryState, Error> {
    use vw_agent_apps::providers::{
        live::{commit::*, write_guard::lock_app},
        omp::{self, OmpFiles},
    };
    use vw_api_types::{agent_apps::AppId, app_providers::omp::OmpActivation, id::ProjectScope};
    let project = ProjectScope::new(tenant.clone(), query.project_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_input"))?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_input"))?;
    if let Some((id, _)) = &request {
        vw_agent_apps::providers::validate_identity(id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_input"))?;
    }
    let authorize = || {
        if request.is_some() {
            super::agent_apps::authorize_aggregation_write(
                policy.as_deref(),
                &headers,
                &project,
                &query.target_host_id,
                AppId::Omp,
                false,
            )
        } else {
            super::agent_apps::authorize_aggregation_read(
                policy.as_deref(),
                &headers,
                &project,
                &query.target_host_id,
                AppId::Omp,
            )
        }
        .map_err(auth_error)
    };
    let authorization = authorize()?;
    let (home, root, principal) = &authorization;
    let _guard = lock_app(AppId::Omp, home).map_err(file_error)?;
    let tx = ProviderTransactions::open_existing(
        root.as_deref().ok_or((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"))?,
        ExecutionHostId::new(&query.target_host_id).map_err(file_error)?,
    )
    .map_err(file_error)?
    .ok_or((StatusCode::CONFLICT, "revision_conflict"))?;
    let state = tx.state(AppId::Omp).map_err(file_error)?;
    if state.project_scope.as_ref() != Some(&project) {
        return Err(if state.project_scope.is_some() {
            (StatusCode::FORBIDDEN, "forbidden")
        } else {
            (StatusCode::CONFLICT, "revision_conflict")
        });
    }
    state.require_project(&project).map_err(file_error)?;
    if let Some(pending) = tx.pending(AppId::Omp).map_err(file_error)? {
        if pending.tenant != tenant || pending.target.project_scope.as_ref() != Some(&project) {
            return Err((StatusCode::FORBIDDEN, "forbidden"));
        }
        return Err((StatusCode::CONFLICT, "recovery_pending"));
    }
    let native = omp::read_native_directory(home, None).map_err(read_error)?;
    let revision =
        make_revision(&project, &query.target_host_id, principal, &state, &native.source_revision)?;
    let unchanged = || {
        authorize().is_ok_and(|current| current == authorization)
            && tx.state(AppId::Omp).is_ok_and(|current| current == state)
            && omp::read_native_directory(home, None).is_ok_and(|current| current == native)
    };
    if !unchanged() || tx.pending(AppId::Omp).map_err(file_error)?.is_some() {
        return Err((StatusCode::CONFLICT, "revision_conflict"));
    }
    let Some((id, request)) = &request else {
        return Ok(project_state(native, revision));
    };
    if request.expected_revision != revision {
        return Err((StatusCode::CONFLICT, "revision_conflict"));
    }
    drop(_guard);
    let mut files = OmpFiles::open(home).map_err(file_error)?;
    let (name, pre) = files.snapshot().map_err(file_error)?;
    let exists = omp::declarations(pre.as_deref()).map_err(file_error)?.contains_key(id);
    let (profile, activation) = match &request.change {
        OmpDirectoryChange::Insert { profile } => {
            if exists {
                return Err((StatusCode::CONFLICT, "revision_conflict"));
            }
            (Some(profile), OmpActivation::Add)
        }
        OmpDirectoryChange::Replace { profile } => {
            if !exists {
                return Err((StatusCode::NOT_FOUND, "not_found"));
            }
            (Some(profile), OmpActivation::IfMember)
        }
        OmpDirectoryChange::Remove {} => (None, OmpActivation::Add),
    };
    if let Some(profile) = profile {
        omp::validate(profile).map_err(file_error)?;
        // A directory entry can only carry provider/model configuration. In particular,
        // do not serialize MCP server definitions or execution instructions as unknown YAML.
        if profile.declaration.as_object().is_none_or(|fields| {
            fields.keys().any(|key| {
                ![
                    "name",
                    "baseUrl",
                    "api",
                    "apiKey",
                    "auth",
                    "authHeader",
                    "headers",
                    "models",
                    "modelOverrides",
                    "compat",
                    "disableStrictTools",
                    "guardrailIdentifier",
                    "requestMetadata",
                    "remoteCompaction",
                    "discovery",
                ]
                .contains(&key.as_str())
            })
        }) {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"));
        }
    }
    if !exists && matches!(request.change, OmpDirectoryChange::Remove {}) {
        if !unchanged() || tx.pending(AppId::Omp).map_err(file_error)?.is_some() {
            return Err((StatusCode::CONFLICT, "revision_conflict"));
        }
        return Ok(project_state(native, revision));
    }
    let next = omp::patch(pre.as_deref(), id, profile.map(|p| &p.declaration), activation)
        .map_err(file_error)?;
    files
        .apply_project(&tx, &project, &state, name, pre.as_deref(), next, unchanged)
        .map_err(file_error)?;
    let after = omp::read_native_directory(home, None).map_err(read_error)?;
    let revision = make_revision(
        &project,
        &query.target_host_id,
        principal,
        &tx.state(AppId::Omp).map_err(file_error)?,
        &after.source_revision,
    )?;
    Ok(project_state(after, revision))
}
#[cfg(target_os = "macos")]
fn project_state(
    native: vw_agent_apps::providers::omp::OmpNativeDirectory,
    revision: String,
) -> OmpDirectoryState {
    OmpDirectoryState {
        revision,
        profile: native.profile,
        members: native.members,
        default_provider_id: native.default_provider_id,
        default_model_id: native.default_model_id,
    }
}
#[cfg(target_os = "macos")]
fn make_revision(
    project: &vw_api_types::id::ProjectScope,
    host: &str,
    principal: &str,
    state: &vw_agent_apps::providers::live::commit::AppState,
    source: &str,
) -> Result<String, Error> {
    use std::sync::LazyLock;
    static KEY: LazyLock<Option<ring::hmac::Key>> = LazyLock::new(|| {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).ok()?;
        Some(ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &key))
    });
    let key = KEY.as_ref().ok_or((StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"))?;
    let bytes = serde_json::to_vec(&("omp-directory-v1", project, host, principal, state, source))
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"))?;
    Ok(hex::encode(ring::hmac::sign(key, &bytes).as_ref()))
}
#[cfg(target_os = "macos")]
fn file_error(error: vw_agent_apps::providers::live::write_guard::WriteError) -> Error {
    use vw_agent_apps::providers::live::write_guard::WriteError;
    match error {
        WriteError::Conflict => (StatusCode::CONFLICT, "revision_conflict"),
        WriteError::Recovered => (StatusCode::CONFLICT, "recovery_pending"),
        WriteError::InvalidResource => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"),
        WriteError::Io => (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable"),
    }
}
#[cfg(target_os = "macos")]
fn read_error(error: vw_api_types::app_providers::catalog::ProviderReadUnavailable) -> Error {
    use vw_api_types::app_providers::catalog::ProviderReadUnavailable;
    match error {
        ProviderReadUnavailable::Unreadable => {
            (StatusCode::SERVICE_UNAVAILABLE, "target_unavailable")
        }
        ProviderReadUnavailable::InvalidConfiguration => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration")
        }
        _ => (StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"),
    }
}
#[cfg(target_os = "macos")]
fn auth_error(status: StatusCode) -> Error {
    (
        status,
        match status {
            StatusCode::UNAUTHORIZED => "unauthenticated",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::UNPROCESSABLE_ENTITY => "unsupported_target",
            _ => "target_unavailable",
        },
    )
}
