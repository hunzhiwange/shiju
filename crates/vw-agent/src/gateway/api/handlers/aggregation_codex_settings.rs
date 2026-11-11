//! Scoped Codex settings; config, catalog and intent settle in one native transaction.
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use std::path::PathBuf;
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::TenantScope,
};
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    let router =
        router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from));
    match GlobalDatabase::for_home(&crate::global::paths().home) {
        Ok(db) => router.layer(Extension(db)),
        Err(_) => router,
    }
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/{app}/aggregation/codex/settings", get(read).put(write))
        .layer(Extension(Policy(path)))
        .layer(DefaultBodyLimit::max(16 * 1024))
}
type Failure = (StatusCode, &'static str);
fn error((status, code): Failure) -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};
    (
        status,
        Json(ApiErrorBody {
            error: ApiErrorDetail { code: code.into(), message: code.into(), details: None },
        }),
    )
        .into_response()
}
fn respond(
    result: Result<Result<CodexAggregationSettings, Failure>, tokio::task::JoinError>,
) -> Response {
    let mut r = match result {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => error(e),
        Err(_) => error((StatusCode::BAD_GATEWAY, "target_failed")),
    };
    r.headers_mut().insert(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
    r
}
async fn read(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    query: Result<Query<AggregationQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let (Ok(Path(app)), Ok(Query(query))) = (app, query) else {
        return respond(Ok(Err((StatusCode::BAD_REQUEST, "invalid_request"))));
    };
    respond(
        tokio::task::spawn_blocking(move || execute(policy, tenant, db, headers, app, query, None))
            .await,
    )
}
async fn write(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: Result<Path<AppId>, axum::extract::rejection::PathRejection>,
    request: Result<Json<CodexSettingsRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let (Ok(Path(app)), Ok(Json(request))) = (app, request) else {
        return respond(Ok(Err((StatusCode::BAD_REQUEST, "invalid_request"))));
    };
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
fn failure(status: StatusCode) -> Failure {
    (
        status,
        match status {
            StatusCode::BAD_REQUEST => "invalid_request",
            StatusCode::UNAUTHORIZED => "unauthenticated",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::CONFLICT => "revision_conflict",
            StatusCode::UNPROCESSABLE_ENTITY => "invalid_configuration",
            _ => "target_unavailable",
        },
    )
}
#[cfg(not(target_os = "macos"))]
fn execute(
    _: Policy,
    _: TenantScope,
    _: Option<Extension<GlobalDatabase>>,
    _: HeaderMap,
    _: AppId,
    _: AggregationQuery,
    _: Option<CodexSettingsRequest>,
) -> Result<CodexAggregationSettings, Failure> {
    Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_target"))
}
#[cfg(target_os = "macos")]
fn execute(
    policy: Policy,
    tenant: TenantScope,
    db: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    app: AppId,
    query: AggregationQuery,
    request: Option<CodexSettingsRequest>,
) -> Result<CodexAggregationSettings, Failure> {
    use super::aggregation_read_snapshot as snapshot;
    use vw_agent_apps::{
        aggregation::codex_settings,
        providers::live::{
            commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
            stage::{FileChange, WholeFile},
            write_guard::{FilePrivacy, WriteError, lock_app},
        },
    };
    use vw_api_types::id::ProjectScope;
    let project = ProjectScope::new(tenant.clone(), query.project_id)
        .map_err(|_| failure(StatusCode::BAD_REQUEST))?;
    vw_agent_apps::providers::validate_identity(&query.target_host_id)
        .map_err(|_| failure(StatusCode::BAD_REQUEST))?;
    let authorize = |attached| {
        if request.is_some() {
            super::agent_apps::authorize_aggregation_write(
                policy.0.as_deref(),
                &headers,
                &project,
                &query.target_host_id,
                app,
                attached,
            )
        } else {
            super::agent_apps::authorize_aggregation_read(
                policy.0.as_deref(),
                &headers,
                &project,
                &query.target_host_id,
                app,
            )
        }
    };
    let (home, root, principal) = authorize(false).map_err(|status| {
        if status == StatusCode::UNPROCESSABLE_ENTITY {
            (status, "unsupported_target")
        } else {
            failure(status)
        }
    })?;
    if app != AppId::Codex {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "unsupported_app"));
    }
    let root = root.ok_or(failure(StatusCode::SERVICE_UNAVAILABLE))?;
    let tx = ProviderTransactions::open_existing(
        &root,
        ExecutionHostId::new(&query.target_host_id)
            .map_err(|_| failure(StatusCode::BAD_REQUEST))?,
    )
    .map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))?
    .ok_or(failure(StatusCode::CONFLICT))?;
    // Same application mutex serializes both the read snapshot and the real native writer.
    let mut guard = lock_app(app, &home).map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))?;
    let state = tx.state(app).map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))?;
    if state.project_scope.as_ref() != Some(&project) {
        return Err(failure(if state.project_scope.is_some() {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::CONFLICT
        }));
    }
    state.require_project(&project).map_err(|_| failure(StatusCode::FORBIDDEN))?;
    if let Some(pending) = tx.pending(app).map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))? {
        if pending.tenant != tenant
            || pending.target.project_scope.as_ref().is_some_and(|owner| owner != &project)
        {
            return Err(failure(StatusCode::FORBIDDEN));
        }
        return Err((StatusCode::CONFLICT, "recovery_pending"));
    }
    let attached = state.mode.as_ref().is_some_and(|m| m.attached);
    if request.is_some() && attached {
        if authorize(true).map_err(failure)?
            != (home.clone(), Some(root.clone()), principal.clone())
        {
            return Err(failure(StatusCode::FORBIDDEN));
        }
        // Reacquire at the fixed native directory, then verify the complete snapshot.
        drop(guard);
        guard = lock_app(app, &home.join(".codex"))
            .map_err(|_| failure(StatusCode::UNPROCESSABLE_ENTITY))?;
        if tx.state(app).map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))? != state {
            return Err(failure(StatusCode::CONFLICT));
        }
    }
    let db = db.ok_or(failure(StatusCode::SERVICE_UNAVAILABLE))?.0;
    let capture = || {
        snapshot::load(&db, &project, &query.target_host_id, app, &home, &state, None)
            .map_err(failure)
    };
    let captured = capture()?;
    let revision = |s: &snapshot::Snapshot| {
        snapshot::revision(&project, &query.target_host_id, app, &principal, &state, s)
            .map_err(failure)
    };
    let before = revision(&captured)?;
    if let Some(r) = &request {
        if r.expected_revision.trim().is_empty() {
            return Err(failure(StatusCode::BAD_REQUEST));
        }
        if r.expected_revision != before {
            return Err(failure(StatusCode::CONFLICT));
        }
    }
    let (config, catalog) = snapshot::codex_inputs(&captured).map_err(failure)?;
    let mut plan =
        codex_settings::plan(&state, config, catalog, request.as_ref().map(|r| &r.requested))
            .map_err(|e| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    match e {
                        codex_settings::SettingsError::InvalidModel => "invalid_model",
                        codex_settings::SettingsError::InvalidConfiguration => {
                            "invalid_configuration"
                        }
                    },
                )
            })?;
    let check = || {
        if authorize(attached).map_err(|_| WriteError::Conflict)?
            != (home.clone(), Some(root.clone()), principal.clone())
        {
            return Err(WriteError::Conflict);
        }
        if tx.state(app).map_err(|_| WriteError::Io)? != state
            || revision(&capture().map_err(|_| WriteError::Conflict)?)
                .map_err(|_| WriteError::Conflict)?
                != before
        {
            return Err(WriteError::Conflict);
        }
        Ok(())
    };
    if request.is_some() {
        let config_patch = WholeFile::new(config, Some(plan.config));
        let catalog_patch = WholeFile::new(catalog, plan.catalog.clone());
        let mut changes = Vec::new();
        if attached && plan.catalog.is_some() {
            changes.push(FileChange {
                name: std::ffi::OsStr::new("shiju-model-catalog.json"),
                privacy: FilePrivacy::Private,
                patch: &catalog_patch,
            });
            changes.push(FileChange {
                name: std::ffi::OsStr::new("config.toml"),
                privacy: FilePrivacy::Private,
                patch: &config_patch,
            });
        }
        tx.commit_project_checked(
            &project,
            &mut guard,
            &state,
            false,
            &changes,
            PendingTarget {
                codex_aggregation_settings: Some(plan.settings.requested.clone()),
                ..Default::default()
            },
            check,
        )
        .map_err(|e| match e {
            WriteError::Conflict => failure(StatusCode::CONFLICT),
            WriteError::Recovered => (StatusCode::CONFLICT, "recovery_pending"),
            WriteError::InvalidResource => {
                (StatusCode::UNPROCESSABLE_ENTITY, "invalid_configuration")
            }
            WriteError::Io => (StatusCode::BAD_GATEWAY, "target_failed"),
        })?;
        let after = tx.state(app).map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))?;
        let inputs = snapshot::load(&db, &project, &query.target_host_id, app, &home, &after, None)
            .map_err(failure)?;
        plan.settings.revision =
            snapshot::revision(&project, &query.target_host_id, app, &principal, &after, &inputs)
                .map_err(failure)?;
    } else {
        if tx.pending(app).map_err(|_| failure(StatusCode::SERVICE_UNAVAILABLE))?.is_some() {
            return Err((StatusCode::CONFLICT, "recovery_pending"));
        }
        check().map_err(|_| failure(StatusCode::CONFLICT))?;
        plan.settings.revision = before;
    }
    Ok(plan.settings)
}
