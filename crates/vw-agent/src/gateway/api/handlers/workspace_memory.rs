//! Fixed workspace routes; authorization precedes all native filesystem access.
use axum::{Extension, Json, Router, extract::{DefaultBodyLimit, Path, Query}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};
use vw_agent_apps::{apps::lifecycle::operations::{Operation, OperationFailure, Phase}, workspace_paths::PathError};
use vw_api_types::{agent_apps::AppOperationStatus, id::{ProjectId, ProjectScope, TenantScope}, workspace_memory::*};

#[derive(Clone, Default)]
struct State {
    policy: Option<PathBuf>,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
}
struct Job {
    project: ProjectScope,
    host: String,
    actor: String,
    operation: Operation,
    result: Arc<Mutex<Option<Vec<DailyMemorySearchResult>>>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Scope { project_id: ProjectId, target_host_id: String }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    #[serde(flatten)]
    progress: AppOperationStatus,
    results: Option<Vec<DailyMemorySearchResult>>,
}
pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/openclaw/workspace/files/{filename}", get(read_file).put(write_file))
        .route("/agent-apps/openclaw/workspace/daily", get(list))
        .route("/agent-apps/openclaw/workspace/daily/{filename}", get(read_daily).put(write_daily).delete(delete_daily))
        .route("/agent-apps/openclaw/workspace/search", post(search))
        .route("/agent-apps/openclaw/workspace/search/{id}", get(search_result).delete(cancel))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(Extension(State { policy: std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from), ..Default::default() }))
}
fn authorize(state: &State, tenant: TenantScope, headers: &HeaderMap, scope: &Scope, write: bool) -> Result<(super::access::Host, String), StatusCode> {
    super::access::authorize_workspace(state.policy.as_deref(), headers,
        &ProjectScope::new(tenant, scope.project_id.clone()), &scope.target_host_id, write)
}
fn path_error(error: PathError) -> StatusCode {
    match error {
        PathError::UnauthorizedHost => StatusCode::FORBIDDEN,
        PathError::InvalidPath | PathError::UnsafeEntry => StatusCode::UNPROCESSABLE_ENTITY,
        PathError::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
// Native source commands use this fixed application directory. It is derived
// solely from the dispatcher's trusted executor HOME, never request paths.
fn root(host: &super::access::Host) -> PathBuf { host.home.join(".openclaw/workspace") }

async fn read_file(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(filename): Path<WorkspaceFilename>) -> Result<Json<WorkspaceFileContent>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, _) = authorize(&state, tenant, &headers, &scope, false)?;
        vw_agent_apps::workspace_files::read_workspace_file(&scope.target_host_id, &host.id, &host.home, &root(&host), filename).map_err(path_error).map(|content| Json(WorkspaceFileContent(content)))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn write_file(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(filename): Path<WorkspaceFilename>, Json(body): Json<WorkspaceWriteRequest>) -> Result<StatusCode, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, _) = authorize(&state, tenant, &headers, &scope, true)?;
        vw_agent_apps::workspace_files::write_workspace_file(&scope.target_host_id, &host.id, &host.home, &root(&host), filename, &body.content).map_err(path_error).map(|_| StatusCode::NO_CONTENT)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn read_daily(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(filename): Path<DailyMemoryFilename>) -> Result<Json<WorkspaceFileContent>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, _) = authorize(&state, tenant, &headers, &scope, false)?;
        vw_agent_apps::daily_memory_files::read_daily_memory_file(&scope.target_host_id, &host.id, &host.home, &root(&host), &filename).map_err(path_error).map(|content| Json(WorkspaceFileContent(content)))
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn write_daily(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(filename): Path<DailyMemoryFilename>, Json(body): Json<WorkspaceWriteRequest>) -> Result<StatusCode, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, _) = authorize(&state, tenant, &headers, &scope, true)?;
        vw_agent_apps::daily_memory_files::write_daily_memory_file(&scope.target_host_id, &host.id, &host.home, &root(&host), &filename, &body.content).map_err(path_error).map(|_| StatusCode::NO_CONTENT)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

async fn delete_daily(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(filename): Path<DailyMemoryFilename>) -> Result<StatusCode, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, _) = authorize(&state, tenant, &headers, &scope, true)?;
        vw_agent_apps::daily_memory_files::delete_daily_memory_file(&scope.target_host_id, &host.id, &host.home, &root(&host), &filename).map_err(path_error)?;
        Ok(StatusCode::NO_CONTENT)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn list(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>) -> Result<Json<Vec<DailyMemoryFileInfo>>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, _) = authorize(&state, tenant, &headers, &scope, false)?;
        vw_agent_apps::daily_memory_list::list_daily_memory_files(&scope.target_host_id, &host.id, &host.home, &root(&host)).map(Json).map_err(path_error)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn search(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Json(body): Json<DailyMemorySearchRequest>) -> Result<Json<SearchResponse>, StatusCode> {
    let (host, actor) = authorize(&state, tenant.clone(), &headers, &scope, false)?;
    let op = Operation::new();
    let worker = op.clone();
    let result = Arc::new(Mutex::new(None));
    let output = result.clone();
    let id = uuid::Uuid::new_v4().to_string();
    let mut jobs = state.jobs.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // Retain results until explicit consumption/deletion; never evict live jobs.
    if jobs.len() >= 128 { return Err(StatusCode::TOO_MANY_REQUESTS); }
    let target = scope.target_host_id.clone();
    op.start_task(async move {
        let rows = tokio::task::spawn_blocking(move || {
            vw_agent_apps::daily_memory_search::search_daily_memory_files(&target, &host.id, &host.home, &root(&host), &body.query, &worker)
        }).await.map_err(|_| OperationFailure::Failed)?;
        match rows {
            Ok(rows) => { *output.lock().map_err(|_| OperationFailure::Failed)? = Some(rows); Ok(()) }
            Err(vw_agent_apps::daily_memory_search::SearchError::Cancelled) => Err(OperationFailure::Cancelled),
            Err(_) => Err(OperationFailure::Failed),
        }
    }).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    jobs.insert(id.clone(), Job { project: ProjectScope::new(tenant, scope.project_id), host: scope.target_host_id, actor, operation: op.clone(), result });
    Ok(Json(SearchResponse { progress: super::lifecycle::status(id, &op), results: None }))
}
fn owned(state: &State, tenant: TenantScope, headers: &HeaderMap, scope: &Scope, id: &str) -> Result<(Operation, Arc<Mutex<Option<Vec<DailyMemorySearchResult>>>>), StatusCode> {
    let (_, actor) = authorize(state, tenant.clone(), headers, scope, false)?;
    let jobs = state.jobs.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let job = jobs.get(id).ok_or(StatusCode::NOT_FOUND)?;
    if job.project != ProjectScope::new(tenant, scope.project_id.clone()) || job.host != scope.target_host_id || job.actor != actor { return Err(StatusCode::NOT_FOUND); }
    Ok((job.operation.clone(), job.result.clone()))
}
async fn search_result(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(id): Path<String>) -> Result<Json<SearchResponse>, StatusCode> {
    let (op, result) = owned(&state, tenant, &headers, &scope, &id)?;
    let rows = if op.snapshot().phase == Phase::Terminal && !op.is_cancelled() {
        result.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.clone()
    } else { None };
    Ok(Json(SearchResponse { progress: super::lifecycle::status(id, &op), results: rows }))
}
async fn cancel(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<Scope>, Path(id): Path<String>) -> Result<Json<SearchResponse>, StatusCode> {
    let (op, _) = owned(&state, tenant, &headers, &scope, &id)?;
    op.cancel_and_wait().await;
    state.jobs.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.remove(&id);
    Ok(Json(SearchResponse { progress: super::lifecycle::status(id, &op), results: None }))
}
#[cfg(test)]
#[path = "workspace_memory_tests.rs"]
mod tests;
