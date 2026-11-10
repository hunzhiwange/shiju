//! Authorize application mode changes and prepare gateway-owned routing.
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
use super::super::app_routing::RoutingControl;
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
    #[cfg(target_os = "macos")] (runtime, control): (
        Option<Extension<ProviderRouteRuntime>>,
        Option<Extension<RoutingControl>>,
    ),
    #[cfg(not(target_os = "macos"))] runtime: Option<Extension<ProviderRouteRuntime>>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ModeApplyRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else { return StatusCode::BAD_REQUEST.into_response() };
    let result = tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "macos")]
        if (!cfg!(test) || runtime.is_none())
            && let ModeApplyAction::Proxy { provider_id } = &request.action
        {
            let Some(project_id) = request.project_id else {
                super::super::agent_apps::provider_editor_context(
                    policy.0.as_deref(),
                    &headers,
                    &tenant,
                    &request.target_host_id,
                    true,
                )?;
                return Err(StatusCode::FORBIDDEN);
            };
            let project = vw_api_types::id::ProjectScope::new(tenant, project_id)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let Extension(control) = control.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let (root, principal) =
                control.authorize_mode(&headers, &project, &request.target_host_id, app)?;
            let scope = vw_agent_apps::providers::mode_apply::RouteScope {
                tenant: project.tenant_scope(),
                host: &request.target_host_id,
                app,
            };
            let store = super::store(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            prepare_proxy_route(
                &control.routing,
                &store,
                &root,
                &project,
                &scope,
                &principal,
                provider_id,
            )?;
            return Ok(publication_unavailable());
        }
        let (home, root) = super::super::agent_apps::provider_editor_context(
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
fn require_direct(row: &serde_json::Value) -> Result<(), StatusCode> {
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
fn prepare_proxy_route(
    listener: &crate::app_routing::RoutingListener,
    store: &crate::app_providers::AppProviderStore,
    root: &std::path::Path,
    project: &vw_api_types::id::ProjectScope,
    scope: &vw_agent_apps::providers::mode_apply::RouteScope<'_>,
    principal: &str,
    provider_id: &str,
) -> Result<(), StatusCode> {
    use crate::agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore};
    use sha2::{Digest, Sha256};
    use vw_agent_apps::{
        auth::claude_keychain::MacOsKeychain,
        providers::{
            claude_login::LoginMaterial,
            live::commit::{ExecutionHostId, ProviderTransactions},
            mode_apply::observe_claude_route,
        },
    };
    use vw_api_types::agent_apps_auth::{AuthErrorCode, AuthService};

    if scope.app != AppId::Claude {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    vw_agent_apps::providers::validate_identity(provider_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let host = ExecutionHostId::new(scope.host).map_err(|_| StatusCode::BAD_REQUEST)?;
    let tx = ProviderTransactions::open_existing(root, host)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::CONFLICT)?;
    let _directory_lock = tx.lock_claude_directory().map_err(super::paths::directory_error)?;
    let state = observe_claude_route(&tx, scope.tenant).map_err(map_error)?;
    let direct_id = &state.pointer.as_ref().ok_or(StatusCode::CONFLICT)?.id;
    let read_row =
        |id| store.claude_record(scope.tenant, id).map_err(|_| StatusCode::SERVICE_UNAVAILABLE);
    let direct = read_row(direct_id.as_str())?.ok_or(StatusCode::CONFLICT)?;
    require_direct(&direct)?;
    let row = read_row(provider_id)?.ok_or(StatusCode::NOT_FOUND)?;
    let account_scope = AuthAccountScope {
        tenant: scope.tenant,
        principal,
        host: scope.host,
        service: AuthService::ClaudeOfficial,
    };
    let account_error = |error| match error {
        AuthErrorCode::Forbidden | AuthErrorCode::IdentityMismatch => StatusCode::FORBIDDEN,
        AuthErrorCode::Unsupported => StatusCode::UNPROCESSABLE_ENTITY,
        AuthErrorCode::AccountInUse | AuthErrorCode::Conflict | AuthErrorCode::ReauthRequired => {
            StatusCode::CONFLICT
        }
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    let account = store
        .claude_account_binding(account_scope, provider_id)
        .map_err(account_error)?
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let accounts = store.claude_accounts(root).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let lease = accounts.managed_official_lease(account_scope, &account).map_err(account_error)?;
    let metadata = accounts.read_metadata(&account_scope, &account).map_err(account_error)?;
    let directory = accounts.claude_managed_directory(&lease).map_err(account_error)?;
    let material = LoginMaterial::from_account(directory.path(), &MacOsKeychain)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let identity =
        material.identity().map_err(|_| StatusCode::FORBIDDEN)?.ok_or(StatusCode::CONFLICT)?;
    let digest: [u8; 32] = Sha256::digest(identity.as_bytes()).into();
    if digest != metadata.external_identity_sha256 {
        return Err(StatusCode::FORBIDDEN);
    }
    let generation = material
        .credential_generation()
        .map_err(|_| StatusCode::CONFLICT)?
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if metadata.authenticated_at.0 <= 0 || generation <= metadata.authenticated_at {
        return Err(StatusCode::CONFLICT);
    }
    if read_row(provider_id)?.as_ref() != Some(&row)
        || read_row(direct_id.as_str())?.as_ref() != Some(&direct)
        || observe_claude_route(&tx, scope.tenant).map_err(map_error)? != state
    {
        return Err(StatusCode::CONFLICT);
    }
    let account_key =
        AuthAccountStore::managed_key(&account_scope, &account).map_err(account_error)?;
    let binding = crate::app_routing::VerifiedRouteBinding::new(
        project.clone(),
        scope.host.into(),
        scope.app,
        account_key,
        generation.0 as u64,
    );
    listener.prepare_route(&binding).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

#[cfg(target_os = "macos")]
fn publication_unavailable() -> Response {
    use vw_api_types::error::{ApiErrorBody, ApiErrorDetail};

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiErrorBody {
            error: ApiErrorDetail {
                code: "route_publication_unavailable".into(),
                message: "Route preparation succeeded; publication is unavailable".into(),
                details: None,
            },
        }),
    )
        .into_response()
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
