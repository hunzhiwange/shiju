//! Authorized native Hermes memory and read-only provider model projection.
use axum::{Extension, Json, Router, extract::{Path, Query, DefaultBodyLimit}, http::{HeaderMap, StatusCode}, routing::{get, put}};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{hermes_memory::*, id::{ProjectId, ProjectScope, TenantScope}};
use vw_agent_apps::providers::{hermes, live::{commit::{ProviderTransactions, ExecutionHostId}, write_guard::WriteError}};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Scope { project_id: ProjectId, target_host_id: String }

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/hermes/memory/{kind}", get(read).put(write))
        .route("/agent-apps/hermes/memory/{kind}/enabled", put(enabled))
        .route("/agent-apps/hermes/memory-limits", get(limits))
        .route("/agent-apps/hermes/model", get(model))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
fn authorize(policy: &Policy, tenant: TenantScope, headers: &HeaderMap, scope: &Scope, write: bool) -> Result<super::access::Host, StatusCode> {
    super::access::authorize_hermes_memory(policy.0.as_deref(), headers,
        &ProjectScope::new(tenant, scope.project_id.clone()), &scope.target_host_id, write)
}
fn write_error(error: WriteError) -> StatusCode {
    match error {
        WriteError::Recovered | WriteError::Conflict => StatusCode::CONFLICT,
        WriteError::InvalidResource => StatusCode::UNPROCESSABLE_ENTITY,
        WriteError::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
fn path_error(error: vw_agent_apps::workspace_paths::PathError) -> StatusCode {
    use vw_agent_apps::workspace_paths::PathError;
    match error {
        PathError::UnauthorizedHost => StatusCode::FORBIDDEN,
        PathError::InvalidPath | PathError::UnsafeEntry => StatusCode::UNPROCESSABLE_ENTITY,
        PathError::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
fn memory(host: &super::access::Host, target: &str, create: bool) -> Result<vw_agent_apps::hermes_memory::HermesMemory, StatusCode> {
    // Same executor environment and path semantics as the provider's native reader.
    let root = std::env::var_os("HERMES_HOME").filter(|v| !v.to_string_lossy().trim().is_empty())
        .map(|v| PathBuf::from(v.to_string_lossy().trim())).unwrap_or_else(|| host.home.join(".hermes"));
    vw_agent_apps::hermes_memory::HermesMemory::open(target, &host.id, &host.home, &root, create).map_err(path_error)
}
async fn read(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Path(k): Path<HermesMemoryKind>) -> Result<Json<HermesMemoryResponse>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, t, &h, &s, false)?;
        memory(&host, &s.target_host_id, false)?.read_memory(k).map(Json).map_err(path_error)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn write(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Path(k): Path<HermesMemoryKind>, Json(b): Json<HermesMemoryWriteRequest>) -> Result<Json<HermesWriteOutcome>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, t, &h, &s, true)?;
        memory(&host, &s.target_host_id, true)?.write_memory(k, &b.content).map_err(path_error)?;
        Ok(Json(HermesWriteOutcome { backup_path: None }))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn limits(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<HermesMemoryLimits>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, t, &h, &s, false)?;
        let bytes = hermes::read(&host.home).map_err(write_error)?;
        vw_agent_apps::hermes_memory_limits::read_memory_limits(bytes.as_deref()).map(Json).map_err(write_error)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn model(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>) -> Result<Json<Option<HermesModelSnapshot>>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, t, &h, &s, false)?;
        let bytes = hermes::read(&host.home).map_err(write_error)?;
        hermes::model_snapshot(bytes.as_deref()).map(Json).map_err(write_error)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn enabled(Extension(p): Extension<Policy>, Extension(t): Extension<TenantScope>, h: HeaderMap, Query(s): Query<Scope>, Path(k): Path<HermesMemoryKind>, Json(b): Json<HermesMemoryEnabledRequest>) -> Result<Json<HermesWriteOutcome>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let host = authorize(&p, t.clone(), &h, &s, true)?;
        let root = host.credential_root.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
        let mut files = hermes::HermesFiles::open(&host.home).map_err(write_error)?;
        let tx = ProviderTransactions::new(&root, ExecutionHostId::new(&host.id).map_err(write_error)?).map_err(write_error)?;
        if tx.recover(&mut files.guard).map_err(write_error)?.is_some() { return Err(StatusCode::CONFLICT); }
        let expected = tx.state(vw_api_types::agent_apps::AppId::Hermes).map_err(write_error)?;
        vw_agent_apps::hermes_memory_limits::set_memory_enabled(&mut files, &tx, &t, &expected, k, b.enabled,
            || authorize(&p, t.clone(), &h, &s, true).is_ok()).map_err(write_error)?;
        Ok(Json(HermesWriteOutcome { backup_path: None }))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
#[cfg(test)]
#[path = "hermes_memory_tests.rs"]
mod tests;
