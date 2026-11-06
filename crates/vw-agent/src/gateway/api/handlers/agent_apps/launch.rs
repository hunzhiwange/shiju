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
fn router_with_terminal(
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
                    search_paths: host.search_paths,
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
                    let prepared = crate::app_providers::AppProviderStore::new(db)
                        .and_then(|store| store.prepare_launch(&tenant, &vw_api_types::app_providers::ProviderLaunchRequest { provider_id, launch: request.launch.clone() }, &host.id, &observed.installations, &[root], false, Path::new("/tmp")))
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                    Some(context.sessions.start(scope, move |ready| {
                        let transport = vw_agent_apps::apps::launch::terminal_session::TerminalSession::connect_with(Path::new("/tmp"), |_, material| terminal(preferred.as_deref(), Path::new("/bin/sh"), material))?;
                        prepared.run_with(|command| transport.run(command, move || { let _ = ready.send(()); }))
                    })?)
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
                let material = validate_launch_request(&request.launch, &host.id, &observed.installations, &[root], false).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                (context.terminal)(preferences.preferred_terminal.as_deref(), Path::new("/bin/sh"), &material).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                None
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
