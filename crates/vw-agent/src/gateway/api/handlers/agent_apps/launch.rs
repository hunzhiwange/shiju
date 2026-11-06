//! Terminal launches are resolved on the granted local execution host.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Query},
    http::{HeaderMap, StatusCode},
    routing::post,
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
struct Context {
    policy: Option<PathBuf>,
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
        .layer(DefaultBodyLimit::max(32768))
        .layer(Extension(Context { policy, terminal }))
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
        let host = super::access::authorize_launch(
            context.policy.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
        )?;
        if request.provider_id.is_some()
            || request.account_id.is_some()
            || request.launch.skip_permissions
        {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
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
            let _ = (host, database);
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
            let material = validate_launch_request(
                &request.launch,
                &host.id,
                &observed.installations,
                &[root],
                false,
            )
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            let db = match database {
                Some(Extension(db)) => db,
                None => GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            };
            let store =
                GatewaySettingsStore::new(db).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let preferences: vw_api_types::agent_apps::AppHostPreferences = store
                .get(&tenant, "agent_apps_host", &host.id)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .unwrap_or_default();
            (context.terminal)(
                preferences.preferred_terminal.as_deref(),
                Path::new("/bin/sh"),
                &material,
            )
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            Ok(Json(LaunchResult { installation_id: request.launch.installation_id }))
        }
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
#[cfg(all(test, unix))]
#[path = "launch_tests.rs"]
mod tests;
