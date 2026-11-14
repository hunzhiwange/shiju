//! Authorized executor-local Claude controls; never accept paths or arbitrary JSON.
use axum::{Extension, Json, Router, extract::{Query, DefaultBodyLimit},
    http::{HeaderMap, StatusCode}, routing::{get, post}};
use std::path::PathBuf;
use vw_api_types::{agent_apps_skills::claude_api::*, id::{ProjectScope, TenantScope}};

#[derive(Clone)]
struct Policy(Option<PathBuf>);

pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}

pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/claude/config-status", get(status))
        .route("/agent-apps/claude/config-marker", post(marker))
        .route("/agent-apps/claude/onboarding", post(onboarding))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(Policy(path)))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn status(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap, Query(scope): Query<ClaudeConfigQuery>,
) -> Result<Json<ClaudeConfigResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let context = context(&policy, &tenant, &headers, &scope, ClaudeConfigPermission::Read)?;
        read_status(context, tenant, scope)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.map(Json)
}

async fn marker(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap, Json(request): Json<ClaudeConfigRequest>,
) -> Result<Json<ClaudeConfigChangeResponse>, StatusCode> {
    change(policy, tenant, headers, request, ClaudeConfigPermission::Marker).await
}
async fn onboarding(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap, Json(request): Json<ClaudeConfigRequest>,
) -> Result<Json<ClaudeConfigChangeResponse>, StatusCode> {
    change(policy, tenant, headers, request, ClaudeConfigPermission::Onboarding).await
}
async fn change(policy: Policy, tenant: TenantScope, headers: HeaderMap,
    request: ClaudeConfigRequest, permission: ClaudeConfigPermission,
) -> Result<Json<ClaudeConfigChangeResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let context = context(&policy, &tenant, &headers, &request.scope, permission)?;
        write(context, tenant, request, permission)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.map(Json)
}
fn context(policy: &Policy, tenant: &TenantScope, headers: &HeaderMap,
    scope: &ClaudeConfigQuery, permission: ClaudeConfigPermission,
) -> Result<(PathBuf, PathBuf), StatusCode> {
    let project = ProjectScope::new(tenant.clone(), scope.project_id.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    super::super::agent_apps::authorize_claude_config(policy.0.as_deref(), headers,
        &project, &scope.target_host_id, permission)
}

#[cfg(target_os = "macos")]
use vw_agent_apps::{providers::live::{commit::{ExecutionHostId, ProviderTransactions},
    write_guard::WriteError}, skills::{claude_status, claude_toggle, claude_onboarding}};

#[cfg(target_os = "macos")]
fn transaction(root: &std::path::Path, host: &str) -> Result<ProviderTransactions, StatusCode> {
    ProviderTransactions::new(root, ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?)
        .map_err(|e| error(e.into()))
}
#[cfg(target_os = "macos")]
fn error(e: claude_status::ClaudeStatusError) -> StatusCode {
    match e {
        claude_status::ClaudeStatusError::InvalidConfig => StatusCode::UNPROCESSABLE_ENTITY,
        claude_status::ClaudeStatusError::Target(WriteError::Conflict) => StatusCode::CONFLICT,
        claude_status::ClaudeStatusError::Target(WriteError::InvalidResource) => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}
#[cfg(target_os = "macos")]
fn read_status((home, root): (PathBuf, PathBuf), tenant: TenantScope, scope: ClaudeConfigQuery)
    -> Result<ClaudeConfigResponse, StatusCode> {
    let tx = transaction(&root, &scope.target_host_id)?;
    let status = match claude_status::get_claude_plugin_status(&home, &tx, &tenant,
        &scope.target_host_id).map_err(error)? {
        claude_status::ClaudeConfigStatus::Missing => ClaudeConfigStatus::Missing,
        claude_status::ClaudeConfigStatus::Invalid => ClaudeConfigStatus::Invalid,
        claude_status::ClaudeConfigStatus::Valid(c) => ClaudeConfigStatus::Valid {
            primary_api_key_present: c.primary_api_key_present, managed: c.managed },
    };
    Ok(ClaudeConfigResponse { scope, status })
}
#[cfg(target_os = "macos")]
fn write((home, root): (PathBuf, PathBuf), tenant: TenantScope, request: ClaudeConfigRequest,
    permission: ClaudeConfigPermission) -> Result<ClaudeConfigChangeResponse, StatusCode> {
    let tx = transaction(&root, &request.scope.target_host_id)?;
    let host = &request.scope.target_host_id;
    let enabled = matches!(request.action, ClaudeConfigAction::Enable);
    let changed = match permission {
        ClaudeConfigPermission::Marker => claude_toggle::apply_claude_plugin_config(&home, &tx, &tenant, host, enabled),
        ClaudeConfigPermission::Onboarding if enabled => claude_onboarding::apply_claude_onboarding_skip(&home, &tx, &tenant, host),
        ClaudeConfigPermission::Onboarding => claude_onboarding::clear_claude_onboarding_skip(&home, &tx, &tenant, host),
        ClaudeConfigPermission::Read => return Err(StatusCode::BAD_REQUEST),
    }.map_err(error)?;
    Ok(ClaudeConfigChangeResponse { scope: request.scope, changed })
}
#[cfg(not(target_os = "macos"))]
fn read_status(_: (PathBuf, PathBuf), _: TenantScope, _: ClaudeConfigQuery)
    -> Result<ClaudeConfigResponse, StatusCode> { Err(StatusCode::NOT_IMPLEMENTED) }
#[cfg(not(target_os = "macos"))]
fn write(_: (PathBuf, PathBuf), _: TenantScope, _: ClaudeConfigRequest, _: ClaudeConfigPermission)
    -> Result<ClaudeConfigChangeResponse, StatusCode> { Err(StatusCode::NOT_IMPLEMENTED) }
