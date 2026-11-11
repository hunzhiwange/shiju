//! Read-only aggregation API on the authorized execution host.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::TenantScope,
};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    // Resolve the existing gateway database once during router assembly. Reads
    // use that handle and never initialize or migrate storage per request.
    let router =
        router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)),
        Err(_) => router,
    }
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/{app}/aggregation/state", get(read))
        .route("/agent-apps/{app}/aggregation/preview", post(preview))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(Extension(Policy(policy)))
}
async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    query: Result<Query<AggregationQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let Ok(Path(app)) = app else { return error(StatusCode::BAD_REQUEST) };
    let Ok(Query(query)) = query else { return error(StatusCode::BAD_REQUEST) };
    respond(
        tokio::task::spawn_blocking(move || execute(policy, tenant, db, headers, app, query, None))
            .await,
    )
}
async fn preview(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    request: Result<Json<AggregationPreviewRequest>, JsonRejection>,
) -> Response {
    let Ok(Path(app)) = app else { return error(StatusCode::BAD_REQUEST) };
    let Ok(Json(request)) = request else { return error(StatusCode::BAD_REQUEST) };
    let query = AggregationQuery {
        target_host_id: request.target_host_id.clone(),
        project_id: request.project_id.clone(),
    };
    respond(
        tokio::task::spawn_blocking(move || {
            execute(policy, tenant, db, headers, app, query, Some(request))
        })
        .await,
    )
}
fn respond(result: Result<Result<Response, StatusCode>, tokio::task::JoinError>) -> Response {
    let mut response = match result {
        Ok(Ok(response)) => response,
        Ok(Err(status)) => error(status),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    response
}
fn error(status: StatusCode) -> Response {
    let code = match status {
        StatusCode::BAD_REQUEST => "invalid_input",
        StatusCode::UNAUTHORIZED => "unauthenticated",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::CONFLICT => "revision_conflict",
        StatusCode::UNPROCESSABLE_ENTITY => "unsupported_target",
        _ => "target_unavailable",
    };
    error_with_code(status, code)
}
fn error_with_code(status: StatusCode, code: &str) -> Response {
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
    _policy: Policy,
    _tenant: TenantScope,
    _db: Option<Extension<GlobalDatabase>>,
    _headers: HeaderMap,
    _app: AppId,
    _query: AggregationQuery,
    _request: Option<AggregationPreviewRequest>,
) -> Result<Response, StatusCode> {
    Err(StatusCode::UNPROCESSABLE_ENTITY)
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: AppId,
    query: AggregationQuery,
    request: Option<AggregationPreviewRequest>,
) -> Result<Response, StatusCode> {
    use vw_agent_apps::providers::{
        live::commit::{ExecutionHostId, ProviderTransactions},
        mode_state,
    };
    use vw_api_types::id::ProjectScope;
    let project =
        ProjectScope::new(tenant.clone(), query.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let (home, credential_root, principal) = super::agent_apps::authorize_aggregation_read(
        policy.0.as_deref(),
        &headers,
        &project,
        &query.target_host_id,
        app,
    )?;
    if !matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop) {
        return Ok(error_with_code(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"));
    }
    if let Some(request) = &request {
        if app != AppId::Codex && request.catalog_policy != CatalogPolicy::Preserve {
            return Err(StatusCode::BAD_REQUEST);
        }
        if let Some(id) = &request.default_provider_id {
            vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
        }
    }
    let root = credential_root.as_deref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    // The app-wide in-process lock is read-only and does not create any files.
    let _guard = vw_agent_apps::providers::live::write_guard::lock_app(app, &home)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let tx = ProviderTransactions::open_existing(
        root,
        ExecutionHostId::new(&query.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .ok_or(StatusCode::CONFLICT)?;
    let state = tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // Project ownership is persisted by the write owner, never claimed by a GET.
    if state.project_scope.as_ref() != Some(&project) {
        return Err(if state.project_scope.is_some() {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::CONFLICT
        });
    }
    let observed = mode_state::observe(&state, &tenant).map_err(|_| StatusCode::FORBIDDEN)?;
    state.require_project(&project).map_err(|_| StatusCode::CONFLICT)?;
    if let Some(pending) = tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
        if pending.tenant != tenant
            || pending.target.project_scope.as_ref().is_some_and(|owner| owner != &project)
        {
            return Err(StatusCode::FORBIDDEN);
        }
        return Ok(error_with_code(StatusCode::CONFLICT, "recovery_pending"));
    }
    let db = db.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let capture = || {
        super::aggregation_read_snapshot::load(
            &db,
            &project,
            &query.target_host_id,
            app,
            &home,
            &state,
            request.as_ref().and_then(|r| r.default_provider_id.as_deref()),
        )
    };
    let snapshot = match capture() {
        Ok(snapshot) => snapshot,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return Ok(error_with_code(StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"));
        }
        Err(status) => return Err(status),
    };
    let revision = super::aggregation_read_snapshot::revision(
        &project,
        &query.target_host_id,
        app,
        &principal,
        &state,
        &snapshot,
    )?;
    let projected = super::aggregation_read_snapshot::project(
        app,
        &tenant,
        &state,
        &snapshot,
        request.as_ref(),
    )?;
    // Detect concurrent executor commits or resource updates before exposing a token.
    let after = match capture() {
        Ok(snapshot) => snapshot,
        Err(StatusCode::UNPROCESSABLE_ENTITY) => {
            return Ok(error_with_code(StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration"));
        }
        Err(status) => return Err(status),
    };
    if tx.state(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? != state
        || tx.pending(app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.is_some()
        || super::aggregation_read_snapshot::revision(
            &project,
            &query.target_host_id,
            app,
            &principal,
            &state,
            &after,
        )? != revision
    {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(request) = request {
        let token = if projected.issues.iter().any(|issue| issue.blocking) {
            None
        } else {
            Some(super::aggregation_read_snapshot::token(
                &principal,
                &project,
                &query.target_host_id,
                app,
                &revision,
                &request,
            )?)
        };
        Ok(Json(AggregationPreview {
            revision,
            preview_token: token,
            default_provider_id: projected.default_provider_id,
            members: projected.members,
            issues: projected.issues,
            notices: projected.notices,
        })
        .into_response())
    } else {
        let attached = observed.attached;
        let mode = if observed.stack.active {
            AggregationMode::Aggregation
        } else if observed.mode == vw_api_types::app_providers::mode_state::ProviderMode::Proxy {
            AggregationMode::Routing
        } else {
            AggregationMode::Direct
        };
        let mut members = projected.members;
        if !attached {
            for member in &mut members {
                member.models.clear();
            }
        }
        Ok(Json(AggregationState {
            revision,
            mode,
            attached,
            default_provider_id: observed.proxy_route,
            direct_provider_id: observed.direct_provider,
            members,
            notices: projected.notices,
            recovery: RecoveryState::None,
        })
        .into_response())
    }
}
