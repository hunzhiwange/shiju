//! Fixed dashboard actions on the authenticated execution host.
use axum::{Extension, Json, Router, extract::{Path, Query}, http::{HeaderMap, StatusCode}, routing::{get, post}};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};
use vw_agent_apps::apps::lifecycle::operations::{Operation, Phase};
use vw_api_types::{hermes_dashboard::*, id::{ProjectScope, TenantScope}};

#[derive(Clone, Default)]
struct State {
    policy: Option<PathBuf>,
    jobs: Arc<Mutex<HashMap<String, (ProjectScope, String, String, Operation)>>>,
}
pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/hermes/dashboard/probe", post(probe))
        .route("/agent-apps/hermes/dashboard/start", post(start))
        .route("/agent-apps/hermes/dashboard/operations/{id}", get(read).delete(cancel))
        .layer(Extension(State { policy: std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from), ..Default::default() }))
}
fn authorize(state: &State, tenant: TenantScope, headers: &HeaderMap, scope: &DashboardScope)
    -> Result<(ProjectScope, super::access::Host, String), StatusCode>
{
    let project = ProjectScope::new(tenant, scope.project_id.clone());
    let (host, principal) = super::access::authorize_hermes_dashboard(state.policy.as_deref(), headers, &project, &scope.target_host_id)?;
    Ok((project, host, principal))
}
async fn probe(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<DashboardScope>) -> Result<Json<DashboardProbe>, StatusCode> {
    let (_, host, _) = authorize(&state, tenant, &headers, &scope)?;
    match vw_agent_apps::hermes_dashboard::probe_dashboard(&scope.target_host_id, &host.id, None).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
        vw_agent_apps::hermes_dashboard::DashboardStatus::Online { url } => Ok(Json(DashboardProbe::Online { url })),
        vw_agent_apps::hermes_dashboard::DashboardStatus::Offline => Ok(Json(DashboardProbe::Offline)),
    }
}
async fn start(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<DashboardScope>) -> Result<Json<DashboardOperation>, StatusCode> {
    let (project, host, principal) = authorize(&state, tenant, &headers, &scope)?;
    #[cfg(not(target_os = "macos"))]
    { let _ = (project, host, principal); Err(StatusCode::NOT_IMPLEMENTED) }
    #[cfg(target_os = "macos")]
    {
        use vw_agent_apps::apps::{discovery::read::{read, LocalReadEnvironment}, launch::{request::validate_launch_request, host::command}};
        use vw_api_types::agent_apps::{AppId, LaunchRequest};
        let material = tokio::task::spawn_blocking(move || {
            let observed = read(AppId::Hermes, &LocalReadEnvironment { host_id: host.id.clone(), home: host.home.clone(), search_paths: host.search_paths.clone(), applications: host.applications }).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            // Never choose arbitrarily between multiple installations.
            if observed.installations.len() != 1 { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
            let request = LaunchRequest { app_id: AppId::Hermes, installation_id: observed.installations[0].id.clone(), cwd: host.home.to_string_lossy().into_owned(), args: vec!["dashboard".into()], skip_permissions: false };
            let material = validate_launch_request(&request, &host.id, &observed.installations, std::slice::from_ref(&host.home), false).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            command(material, &host.home, &host.search_paths).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
        }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;
        let operation = Operation::new();
        let id = uuid::Uuid::new_v4().to_string();
        let mut jobs = state.jobs.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if jobs.len() >= 128 { jobs.retain(|_, (_, _, _, op)| op.snapshot().phase != Phase::Terminal); }
        if jobs.len() >= 128 { return Err(StatusCode::TOO_MANY_REQUESTS); }
        let worker = operation.clone();
        operation.start_task(async move {
            use vw_agent_apps::apps::lifecycle::operations::OperationFailure;
            if worker.is_cancelled() { return Err(OperationFailure::Cancelled); }
            let mut command = tokio::process::Command::from(material);
            command.process_group(0).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).kill_on_drop(true);
            let mut child = command.spawn().map_err(|_| OperationFailure::Failed)?;
            let pid = child.id().ok_or(OperationFailure::Failed)?;
            tokio::select! {
                result = child.wait() => if result.is_ok_and(|s| s.success()) { Ok(()) } else { Err(OperationFailure::Failed) },
                _ = worker.cancelled() => {
                    // Terminate the owned process group, then reap before acknowledging cancellation.
                    unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
                    child.wait().await.map_err(|_| OperationFailure::Failed)?;
                    Err(OperationFailure::Cancelled)
                }
            }
        }).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        jobs.insert(id.clone(), (project, scope.target_host_id, principal, operation.clone()));
        // Running is process progress, never a claim that the HTTP panel is online.
        Ok(Json(super::lifecycle::status(id, &operation)))
    }
}
fn owned(state: &State, tenant: TenantScope, headers: &HeaderMap, scope: &DashboardScope, id: &str) -> Result<Operation, StatusCode> {
    let (project, _, principal) = authorize(state, tenant, headers, scope)?;
    let jobs = state.jobs.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let (owner, host, actor, op) = jobs.get(id).ok_or(StatusCode::NOT_FOUND)?;
    if owner != &project || host != &scope.target_host_id || actor != &principal { return Err(StatusCode::NOT_FOUND); }
    Ok(op.clone())
}
async fn read(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<DashboardScope>, Path(id): Path<String>) -> Result<Json<DashboardOperation>, StatusCode> {
    Ok(Json(super::lifecycle::status(id.clone(), &owned(&state, tenant, &headers, &scope, &id)?)))
}
async fn cancel(Extension(state): Extension<State>, Extension(tenant): Extension<TenantScope>, headers: HeaderMap, Query(scope): Query<DashboardScope>, Path(id): Path<String>) -> Result<Json<DashboardOperation>, StatusCode> {
    let op = owned(&state, tenant, &headers, &scope, &id)?;
    op.cancel_and_wait().await;
    Ok(Json(super::lifecycle::status(id, &op)))
}

#[cfg(test)]
#[path = "hermes_dashboard_tests.rs"]
mod tests;
