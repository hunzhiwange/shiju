//! Authorize application mode changes and prepare gateway-owned routing.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use std::path::PathBuf;
use {vw_api_app_identity::AppId, vw_api_transport::id::TenantScope};
use vw_api_app_provider_environment::mode_apply::*;

#[cfg(target_os = "macos")]
#[derive(Clone)]
pub struct ProviderRouteRuntime(
    pub std::sync::Arc<dyn vw_agent_apps::providers::mode_apply::RouteRuntime>,
);
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub struct ProviderRouteRuntime;
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/mode/apply", post(apply))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(Policy(path)))
}
async fn apply(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<agent_database::GlobalDatabase>>,
    #[cfg(target_os = "macos")] (runtime, host): (
        Option<Extension<ProviderRouteRuntime>>,
        Option<Extension<ModeHost>>,
    ),
    #[cfg(not(target_os = "macos"))] runtime: Option<Extension<ProviderRouteRuntime>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ModeApplyRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    let result = tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "macos")]
        if let Some(response) = host.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0.0.apply(
            policy.0.clone(), tenant.clone(), database.clone(), headers.clone(), app,
            request.clone(), runtime.is_none(),
        )? { return Ok(response); }
        let (home, root) = crate::authorization::provider_editor_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
            true,
        )?;
        execute(&home, &root, tenant, database, runtime, app, request)
            .map(|result| Json(result).into_response())
    })
    .await;
    match result {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => e.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[cfg(target_os = "macos")]
fn execute(
    home: &std::path::Path,
    root: &std::path::Path,
    tenant: TenantScope,
    database: Option<Extension<agent_database::GlobalDatabase>>,
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
    let store = super::store(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
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
    require_direct(&direct_row)?;
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
    require_direct(&row)?;
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
pub fn require_direct(row: &serde_json::Value) -> Result<(), StatusCode> {
    if row.get("category").and_then(serde_json::Value::as_str) == Some("official")
        || row.pointer("/claude/config/account").is_some_and(|v| !v.is_null())
        || row.pointer("/meta/authBinding").is_some()
        || row.pointer("/meta/claudeAccountId").is_some()
        || row.get("managedClaudeLogin").is_some()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn map_error(e: vw_agent_apps::providers::mode_apply::ModeApplyError) -> StatusCode {
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
    _: Option<Extension<agent_database::GlobalDatabase>>,
    _: Option<Extension<ProviderRouteRuntime>>,
    _: AppId,
    _: ModeApplyRequest,
) -> Result<ModeApplyResult, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Host-owned route publication and managed-account preparation.
#[cfg(target_os = "macos")]
pub trait ModePublication: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn apply(&self, policy: Option<PathBuf>, tenant: TenantScope,
        database: Option<Extension<agent_database::GlobalDatabase>>, headers: HeaderMap,
        app: AppId, request: ModeApplyRequest, runtime_missing: bool,
    ) -> Result<Option<Response>, StatusCode>;
}
#[cfg(target_os = "macos")]
#[derive(Clone)]
pub struct ModeHost(pub std::sync::Arc<dyn ModePublication>);
