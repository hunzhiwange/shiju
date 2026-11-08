//! Terminal launches are resolved on the granted local execution host.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path as RoutePath, Query},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use vw_agent_apps::apps::launch::{
    posix::launch_posix_terminal,
    request::{LaunchMaterial, validate_launch_request},
};
use vw_api_types::{
    agent_apps::{LaunchRequest, LaunchResult},
    id::TenantScope,
};
#[derive(Clone)]
pub(super) struct Context {
    pub(super) policy: Option<PathBuf>,
    sessions: super::launch_sessions::Sessions,
    terminal: fn(Option<&str>, &Path, &LaunchMaterial) -> Result<(), String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    #[serde(flatten)]
    launch: LaunchRequest,
    provider_id: Option<String>,
    account_id: Option<String>,
}
pub(super) fn router() -> Router {
    router_with_terminal(
        std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from),
        launch_posix_terminal,
    )
}
pub(super) fn router_with_terminal(
    policy: Option<PathBuf>,
    terminal: fn(Option<&str>, &Path, &LaunchMaterial) -> Result<(), String>,
) -> Router {
    Router::new()
        .route("/agent-apps/launch", post(launch))
        .route("/agent-apps/launch-identities", get(super::launch_identities::list))
        .route("/agent-apps/launch-sessions/{id}", get(session_status))
        .layer(DefaultBodyLimit::max(32768))
        .layer(Extension(Context { policy, terminal, sessions: Default::default() }))
}
async fn launch(
    Extension(context): Extension<Context>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Json(request): Json<Request>,
) -> Result<Json<LaunchResult>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let authorization = super::access::authorize_identity_launch(context.policy.as_deref(), &headers, &tenant, &target.target_host_id)?;
        if (request.provider_id.is_some() && request.account_id.is_some()) || request.launch.skip_permissions { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
        if let Some(id) = &request.provider_id && !authorization.permits(request.launch.app_id, vw_api_types::agent_apps::LaunchIdentityKind::Provider, id) { return Err(StatusCode::FORBIDDEN); }
        if let Some(id) = &request.account_id && !authorization.permits(request.launch.app_id, vw_api_types::agent_apps::LaunchIdentityKind::OfficialAccount, id) { return Err(StatusCode::FORBIDDEN); }
        // An explicitly scoped native slot must not be reachable through ordinary launch.
        let native_service = match request.launch.app_id {
            vw_api_types::agent_apps::AppId::Claude => Some(vw_api_types::agent_apps_auth::AuthService::ClaudeOfficial),
            vw_api_types::agent_apps::AppId::Agy => Some(vw_api_types::agent_apps_auth::AuthService::AgyOauth),
            _ => None,
        };
        if native_service.is_some_and(|service| authorization.native_login_owner(service) == Some(false)) {
            return Err(StatusCode::FORBIDDEN);
        }
        #[cfg(target_os="macos")]
        let agy_owner = context.policy.is_some() && authorization.native_login_owner(vw_api_types::agent_apps_auth::AuthService::AgyOauth) == Some(true);
        let host = authorization.host; let principal = authorization.principal;
        let cwd = PathBuf::from(&request.launch.cwd);
        let root = host.home.canonicalize().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if !cwd.is_absolute()
            || !cwd.canonicalize().is_ok_and(|p| p.is_dir() && p.starts_with(&root))
            || request.launch.cwd.chars().any(char::is_control)
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        #[cfg(not(unix))]
        {
            let _ = (host, database, principal);
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
        #[cfg(unix)]
        {
            let observed = vw_agent_apps::apps::discovery::read::read(
                request.launch.app_id,
                &vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
                    host_id: host.id.clone(),
                    home: host.home.clone(),
                    search_paths: host.search_paths.clone(),
                    applications: host.applications,
                },
            )
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let db = match database {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store =
                GatewaySettingsStore::new(db.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let preferences: vw_api_types::agent_apps::AppHostPreferences = store
                .get(&tenant, "agent_apps_host", &host.id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .unwrap_or_default();
            let session_id = if request.provider_id.is_some() || request.account_id.is_some() {
                let terminal = context.terminal;
                let preferred = preferences.preferred_terminal;
                let scope = super::launch_sessions::Scope { tenant: tenant.clone(), host: host.id.clone(), principal };
                if let Some(provider_id) = request.provider_id {
                    let store=crate::app_providers::AppProviderStore::new(db).map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
                    #[cfg(target_os="macos")]
                    let bound = match request.launch.app_id {
                        vw_api_types::agent_apps::AppId::Claude | vw_api_types::agent_apps::AppId::Codex => {
                            let service = match request.launch.app_id {
                                vw_api_types::agent_apps::AppId::Claude => vw_api_types::agent_apps_auth::AuthService::ClaudeOfficial,
                                _ => vw_api_types::agent_apps_auth::AuthService::CodexOauth,
                            };
                            let binding_scope = crate::agent_apps::auth_accounts::AuthAccountScope { tenant: &tenant, principal: &scope.principal, host: &host.id, service };
                            let material = validate_launch_request(&request.launch,&host.id,&observed.installations,std::slice::from_ref(&root),false)
                                .map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?;
                            store.prepare_bound_official_launch(binding_scope, &provider_id, host.credential_root.as_deref(), material)
                                .map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?
                        }
                        _ => None,
                    };
                    #[cfg(not(target_os="macos"))]
                    let bound: Option<crate::agent_apps::auth_accounts::AuthAccountLaunch> = None;
                    if let Some(prepared) = bound {
                        Some(context.sessions.start(scope, move |ready| {
                            let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                            prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                        })?)
                    } else {
                    #[cfg(target_os="macos")]
                    let agy=if request.launch.app_id==vw_api_types::agent_apps::AppId::Agy {
                        let material=validate_launch_request(&request.launch,&host.id,&observed.installations,std::slice::from_ref(&root),false).map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?;
                        let account_scope = crate::agent_apps::auth_accounts::AuthAccountScope { tenant: &tenant, principal: &scope.principal, host: &host.id, service: vw_api_types::agent_apps_auth::AuthService::AgyOauth };
                        let native = vw_agent_apps::auth::agy_account::AgyNativeTarget { home: &host.home, exclusive_owner: agy_owner.then_some(account_scope) };
                        Some(store.prepare_agy_launch(account_scope,native,&provider_id,host.credential_root.as_deref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?,material).map_err(|_|StatusCode::UNPROCESSABLE_ENTITY)?)
                    }else{None};
                    #[cfg(not(target_os="macos"))]
                    let agy: Option<vw_agent_apps::apps::launch::provider::ProviderLaunch>=None;
                    let prepared=if let Some(prepared)=agy{prepared}else{store.prepare_launch(&tenant, &vw_api_types::app_providers::ProviderLaunchRequest { provider_id, launch: request.launch.clone() }, &host.id, &observed.installations, &[root], false, Path::new("/tmp"))
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?};
                    Some(context.sessions.start(scope, move |ready| {
                        let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                        prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                    })?)
                    }
                } else {
                    let credential_root = host.credential_root.ok_or(StatusCode::NOT_IMPLEMENTED)?;
                    let prepared = crate::agent_apps::auth_accounts::AuthAccountStore::new(db, &credential_root)
                        .and_then(|store| store.prepare_launch(&tenant, &host.id, request.account_id.as_deref().ok_or("account missing")?, &request.launch, &observed.installations, &[root], Path::new("/tmp")))
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                    Some(context.sessions.start(scope, move |ready| {
                        let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                        prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                    })?)
                }
            } else {
                let material = validate_launch_request(&request.launch, &host.id, &observed.installations, &[root.clone()], false).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let mut command = vw_agent_apps::apps::launch::host::command(material, &root, &host.search_paths).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                let terminal = context.terminal;
                let preferred = preferences.preferred_terminal;
                let scope = super::launch_sessions::Scope { tenant, host: host.id, principal };
                Some(context.sessions.start(scope, move |ready| {
                    let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                    let status = transport.run(&mut command, move || { let _ = ready.send(()); }).map_err(|_| "execution-host process failed")?;
                    if !status.success() { return Err("execution-host process exited unsuccessfully".into()); }
                    Ok(())
                })?)
            };
            Ok(Json(LaunchResult { session_id, installation_id: request.launch.installation_id }))
        }
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn session_status(
    Extension(context): Extension<Context>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    RoutePath(id): RoutePath<String>,
) -> Result<Json<vw_api_types::agent_apps::LaunchSessionStatus>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, principal) = super::access::authorize_launch_session(
            context.policy.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
        )?;
        context
            .sessions
            .get(&super::launch_sessions::Scope { tenant, host: host.id, principal }, &id)
            .map(Json)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
#[cfg(all(test, unix))]
#[path = "launch_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "../app_providers/agy_tests.rs"]
mod agy_tests;
