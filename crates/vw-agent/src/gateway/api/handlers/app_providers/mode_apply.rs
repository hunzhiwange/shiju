//! Authorized mode operation with an optional executor-injected routing service.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use std::path::PathBuf;
use vw_api_types::{agent_apps::AppId, app_providers::mode_apply::*, id::TenantScope};
#[cfg(target_os = "macos")]
#[derive(Clone)]
pub(crate) struct ProviderRouteRuntime(
    pub std::sync::Arc<dyn vw_agent_apps::providers::mode_apply::RouteRuntime>,
);
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub(crate) struct ProviderRouteRuntime;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/mode/apply", post(apply))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(Policy(path)))
}
async fn apply(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<crate::global_database::GlobalDatabase>>,
    runtime: Option<Extension<ProviderRouteRuntime>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ModeApplyRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    let result = tokio::task::spawn_blocking(move || {
        let (home, root) = super::super::agent_apps::provider_editor_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            true,
        )?;
        execute(&home, &root, tenant, database, runtime, app, request)
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(e)) => e.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[cfg(target_os = "macos")]
fn execute(
    home: &std::path::Path,
    root: &std::path::Path,
    tenant: TenantScope,
    database: Option<Extension<crate::global_database::GlobalDatabase>>,
    runtime: Option<Extension<ProviderRouteRuntime>>,
    app: AppId,
    request: ModeApplyRequest,
) -> Result<ModeApplyResult, StatusCode> {
    use vw_agent_apps::providers::{claude::ClaudeFile, mode_apply::*};
    if app != AppId::Claude {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if let ModeApplyAction::Proxy { provider_id } = &request.action {
        vw_agent_apps::providers::validate_identity(provider_id)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    let runtime = runtime.as_ref().map(|r| r.0.0.as_ref());
    let scope = RouteScope { tenant: &tenant, host: &request.target_host_id, app };
    if matches!(request.action, ModeApplyAction::Proxy { .. }) {
        runtime.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.prepare(&scope).map_err(map_error)?;
    }
    let database = database.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let store = crate::app_providers::AppProviderStore::new(database)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // Inspect the existing owner before provisioning any native or state directory.
    let tx = super::paths::transactions(root, &request.target_host_id)?;
    let _directory_lock = tx.lock_claude_directory().map_err(super::paths::directory_error)?;
    let directory = tx.claude_directory(&tenant).map_err(super::paths::directory_error)?;
    let state = tx.state(app).map_err(|_| StatusCode::CONFLICT)?;
    vw_agent_apps::providers::mode_state::observe(&state, &tenant)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    // Keep a restorable direct row pinned while attaching. Native-account
    // transitions are owned by the login adapter and are explicitly unsupported.
    let direct_id = state.pointer.as_ref().map(|p| p.id.as_str()).ok_or(StatusCode::CONFLICT)?;
    let direct_row = store
        .claude_record(&tenant, direct_id)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::CONFLICT)?;
    if direct_row.get("category").and_then(serde_json::Value::as_str) == Some("official")
        || direct_row.pointer("/claude/config/account").is_some_and(|v| !v.is_null())
        || direct_row.pointer("/meta/authBinding").is_some()
        || direct_row.pointer("/meta/claudeAccountId").is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let id = match &request.action {
        ModeApplyAction::Proxy { provider_id } => provider_id.as_str(),
        _ => state.pointer.as_ref().map(|p| p.id.as_str()).ok_or(StatusCode::CONFLICT)?,
    };
    vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let row = store
        .claude_record(&tenant, id)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::NOT_FOUND)?;
    // Native account changes remain with the login owner. Do not route a row that
    // would require an unimplemented managed-account transition.
    if row.get("category").and_then(serde_json::Value::as_str) == Some("official")
        || row.pointer("/claude/config/account").is_some_and(|v| !v.is_null())
        || row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/claudeAccountId").is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let settings = row.get("settingsConfig").ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut file = ClaudeFile::open_selected(home, directory.as_ref(), true)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if tx.state(app).map_err(|_| StatusCode::CONFLICT)? != state {
        return Err(StatusCode::CONFLICT);
    }
    ClaudeMode { file: &mut file, transactions: &tx, scope, runtime }
        .apply(&request.action, settings, || {
            store.claude_record(&tenant, id).ok().flatten().as_ref() == Some(&row)
                && store.claude_record(&tenant, direct_id).ok().flatten().as_ref()
                    == Some(&direct_row)
        })
        .map_err(map_error)
}
#[cfg(target_os = "macos")]
fn map_error(e: vw_agent_apps::providers::mode_apply::ModeApplyError) -> StatusCode {
    use vw_agent_apps::providers::{live::write_guard::WriteError, mode_apply::ModeApplyError::*};
    match e {
        Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Ownership => StatusCode::FORBIDDEN,
        InvalidInput => StatusCode::UNPROCESSABLE_ENTITY,
        Write(WriteError::Conflict | WriteError::Recovered) => StatusCode::CONFLICT,
        Write(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
#[cfg(not(target_os = "macos"))]
fn execute(
    _: &std::path::Path,
    _: &std::path::Path,
    _: TenantScope,
    _: Option<Extension<crate::global_database::GlobalDatabase>>,
    _: Option<Extension<ProviderRouteRuntime>>,
    _: AppId,
    _: ModeApplyRequest,
) -> Result<ModeApplyResult, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
