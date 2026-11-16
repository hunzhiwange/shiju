//! Concrete native route publication adapter for provider mode changes.
use super::lifecycle_api::RoutingControl;
use axum::{Extension, Json, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}};
use agent_database::GlobalDatabase;
use vw_api_app_identity::AppId;
use vw_api_transport::TenantScope;
use vw_api_app_provider_environment::mode_apply::*;
use gateway_app_providers::mode_apply::{require_direct, map_error};
#[cfg(target_os = "macos")]
pub struct Host(pub Option<RoutingControl>, pub bool);
#[cfg(target_os = "macos")]
impl gateway_app_providers::mode_apply::ModePublication for Host {
    fn apply(&self, policy: Option<std::path::PathBuf>, tenant: TenantScope,
        database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
        app: AppId, request: ModeApplyRequest, runtime_missing: bool,
    ) -> Result<Option<Response>, StatusCode> {
        let control = self.0.clone().map(Extension);
        #[cfg(target_os = "macos")]
        if (self.1 || runtime_missing)
            && request.project_id.is_some()
            && app == AppId::Claude
        {
            let project = vw_api_transport::ProjectScope::new(
                tenant.clone(),
                request.project_id.clone().ok_or(StatusCode::BAD_REQUEST)?,
            )
            .map_err(|_| StatusCode::BAD_REQUEST)?;
            let Extension(control) = control.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let (root, _) =
                control.authorize_mode(&headers, &project, &request.target_host_id, app)?;
            let (home, provider_root) = gateway_agent_apps::host_context::provider_editor_context(
                policy.as_deref(),
                &headers,
                &tenant,
                &request.target_host_id,
                true,
            )?;
            if root != provider_root {
                return Err(StatusCode::CONFLICT);
            }
            let store =
                gateway_app_providers::store(database.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let third_party = match &request.action {
                ModeApplyAction::Proxy { provider_id } => {
                    vw_agent_apps::providers::validate_identity(provider_id)
                        .map_err(|_| StatusCode::BAD_REQUEST)?;
                    store
                        .claude_record(&tenant, provider_id)
                        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                        .is_some_and(|row| require_direct(&row).is_ok())
                }
                _ => true,
            };
            if third_party {
                let database = match database {
                    Some(Extension(database)) => database,
                    None => agent_database::GlobalDatabase::for_home(
                        &agent_paths::paths().home,
                    )
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
                };
                let scope = vw_api_app_failover::FailoverQueueScope {
                    project,
                    target_host_id: request.target_host_id.clone(),
                    app_id: app,
                };
                return app_routing_services::provider_publication::apply(
                    database,
                    &control.routing,
                    &home,
                    &root,
                    &scope,
                    &request.action,
                )
                .map(|result| Some(Json(result).into_response()));
            }
        }
        #[cfg(target_os = "macos")]
        if (self.1 || runtime_missing)
            && let ModeApplyAction::Proxy { provider_id } = &request.action
        {
            let Some(project_id) = request.project_id.clone() else {
                gateway_agent_apps::host_context::provider_editor_context(
                    policy.as_deref(),
                    &headers,
                    &tenant,
                    &request.target_host_id,
                    true,
                )?;
                return Err(StatusCode::FORBIDDEN);
            };
            let project = vw_api_transport::ProjectScope::new(tenant.clone(), project_id)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let Extension(control) = control.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let (root, principal) =
                control.authorize_mode(&headers, &project, &request.target_host_id, app)?;
            let scope = vw_agent_apps::providers::mode_apply::RouteScope {
                tenant: project.tenant_scope(),
                host: &request.target_host_id,
                app,
            };
            let store = gateway_app_providers::store(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            prepare_proxy_route(
                &control.routing,
                &store,
                &root,
                &project,
                &scope,
                &principal,
                provider_id,
            )?;
            return Ok(Some(publication_unavailable()));
        }
        Ok(None)
    }
}
#[cfg(target_os = "macos")]
fn prepare_proxy_route(
    listener: &app_routing_services::RoutingListener,
    store: &app_provider_services::AppProviderStore,
    root: &std::path::Path,
    project: &vw_api_transport::ProjectScope,
    scope: &vw_agent_apps::providers::mode_apply::RouteScope<'_>,
    principal: &str,
    provider_id: &str,
) -> Result<(), StatusCode> {
    use agent_app_services::auth_accounts::{AuthAccountScope, AuthAccountStore};
    use sha2::{Digest, Sha256};
    use vw_agent_apps::{
        auth::claude_keychain::MacOsKeychain,
        providers::{
            claude_login::LoginMaterial,
            live::commit::{ExecutionHostId, ProviderTransactions},
            mode_apply::observe_claude_route,
        },
    };
    use vw_api_app_auth::{AuthErrorCode, AuthService};

    if scope.app != AppId::Claude {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    vw_agent_apps::providers::validate_identity(provider_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let host = ExecutionHostId::new(scope.host).map_err(|_| StatusCode::BAD_REQUEST)?;
    let tx = ProviderTransactions::open_existing(root, host)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::CONFLICT)?;
    let _directory_lock = tx.lock_claude_directory().map_err(gateway_app_providers::paths::directory_error)?;
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
    let binding = app_routing_services::VerifiedRouteBinding::new(
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
    use {
    vw_api_transport::error::ApiErrorBody,
    vw_api_transport::error::ApiErrorDetail,
};

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
