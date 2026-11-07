//! Explicit read permission is checked before resolving any host path.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::path::PathBuf;
use vw_api_types::{agent_apps::AppId, app_providers::paths::*, id::TenantScope};
#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/{app}/config/paths", post(read).layer(DefaultBodyLimit::max(4096)))
        .route(
            "/app-providers/{app}/config/directory",
            get(directory).put(save_directory).layer(DefaultBodyLimit::max(4096)),
        )
        .layer(Extension(PolicyFile(path)))
}

async fn directory(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    Query(request): Query<ProviderDirectoryQuery>,
) -> Result<Json<ProviderDirectoryStatus>, StatusCode> {
    directory_request(policy, tenant, headers, app, request.target_host_id, None).await
}

async fn save_directory(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ProviderDirectoryUpdate>, JsonRejection>,
) -> Result<Json<ProviderDirectoryStatus>, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    directory_request(policy, tenant, headers, app, request.target_host_id, Some(request.action))
        .await
}

async fn directory_request(
    policy: PolicyFile,
    tenant: TenantScope,
    headers: HeaderMap,
    app: AppId,
    host: String,
    action: Option<DirectoryOverride>,
) -> Result<Json<ProviderDirectoryStatus>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        directory_sync(&policy, &tenant, &headers, app, &host, action)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .map(Json)
}

#[cfg(target_os = "macos")]
fn directory_sync(
    policy: &PolicyFile,
    tenant: &TenantScope,
    headers: &HeaderMap,
    app: AppId,
    host: &str,
    action: Option<DirectoryOverride>,
) -> Result<ProviderDirectoryStatus, StatusCode> {
    let (home, root) = super::super::agent_apps::provider_preview_context(
        policy.0.as_deref(),
        headers,
        tenant,
        host,
    )?;
    if app != AppId::Claude {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    let saved = if let Some(action) = action {
        let (write_home, write_root) = super::super::agent_apps::provider_editor_context(
            policy.0.as_deref(),
            headers,
            tenant,
            host,
            true,
        )?;
        if write_home != home || Some(&write_root) != root.as_ref() {
            return Err(StatusCode::CONFLICT);
        }
        let requested = match &action {
            DirectoryOverride::Set { directory } => Some(directory.as_str()),
            DirectoryOverride::Reset {} => None,
        };
        transactions(&write_root, host)?
            .set_claude_directory(&home, tenant, requested)
            .map_err(directory_error)?
    } else {
        saved_claude_directory(root.as_deref(), host, tenant)?
    };
    let directory_override = saved
        .as_ref()
        .map(|saved| saved.validated_path(&home))
        .transpose()
        .map_err(directory_error)?;
    let paths = vw_agent_apps::providers::paths::read(
        &home,
        app,
        directory_override,
        &vw_agent_apps::providers::paths::execution_environment(),
    )
    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    Ok(ProviderDirectoryStatus { paths, directory_override: directory_override.map(str::to_owned) })
}

#[cfg(not(target_os = "macos"))]
fn directory_sync(
    _: &PolicyFile,
    _: &TenantScope,
    _: &HeaderMap,
    _: AppId,
    _: &str,
    _: Option<DirectoryOverride>,
) -> Result<ProviderDirectoryStatus, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

#[cfg(target_os = "macos")]
use vw_agent_apps::providers::live::{
    claude_directory::ClaudeDirectory,
    commit::{ExecutionHostId, ProviderTransactions},
    write_guard::WriteError,
};

#[cfg(target_os = "macos")]
pub(super) fn directory_error(error: WriteError) -> StatusCode {
    match error {
        WriteError::Conflict => StatusCode::CONFLICT,
        WriteError::InvalidResource => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

#[cfg(target_os = "macos")]
pub(super) fn transactions(
    root: &std::path::Path,
    host: &str,
) -> Result<ProviderTransactions, StatusCode> {
    let host = ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?;
    ProviderTransactions::new(root, host).map_err(directory_error)
}

#[cfg(target_os = "macos")]
pub(super) fn saved_claude_directory(
    root: Option<&std::path::Path>,
    host: &str,
    tenant: &TenantScope,
) -> Result<Option<ClaudeDirectory>, StatusCode> {
    let Some(root) = root else { return Ok(None) };
    let host = ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?;
    ProviderTransactions::open_existing(root, host)
        .and_then(|tx| tx.map(|tx| tx.claude_directory(tenant)).transpose())
        .map(Option::flatten)
        .map_err(directory_error)
}
async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(app): Path<AppId>,
    body: Result<Json<ProviderPathsRequest>, JsonRejection>,
) -> Result<Response, StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        let (home, root) = super::super::agent_apps::provider_preview_context(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &request.target_host_id,
        )?;
        let mut directory_override = request.directory_override;
        #[cfg(target_os = "macos")]
        if app == AppId::Claude && directory_override.is_none() {
            directory_override =
                saved_claude_directory(root.as_deref(), &request.target_host_id, &tenant)?
                    .map(|saved| saved.validated_path(&home).map(str::to_owned))
                    .transpose()
                    .map_err(directory_error)?;
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (&mut directory_override, root);
        let environment = vw_agent_apps::providers::paths::execution_environment();
        vw_agent_apps::providers::paths::read(
            &home,
            app,
            directory_override.as_deref(),
            &environment,
        )
        .map(|value| Json(value).into_response())
        .map_err(|_| StatusCode::BAD_REQUEST)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
