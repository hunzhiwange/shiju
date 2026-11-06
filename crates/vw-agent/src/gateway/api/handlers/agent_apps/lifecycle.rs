//! Fixed lifecycle endpoints; opaque IDs never confer scope or host authority.
use super::access;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vw_agent_apps::apps::{
    discovery::read::LocalReadEnvironment,
    lifecycle::{
        local,
        operations::{Operation, Phase},
    },
};
use vw_api_types::{
    agent_apps::*,
    id::{TaskId, TenantScope},
    task::TaskStatus,
};

#[derive(Clone)]
struct State {
    policy: Option<PathBuf>,
    store: Arc<Mutex<Store>>,
}
#[derive(Default)]
struct Store {
    previews: HashMap<String, Preview>,
    operations: HashMap<String, Running>,
}
struct Preview {
    tenant: TenantScope,
    host: String,
    created: Instant,
    plan: local::Plan,
    write: local::WriteEnvironment,
}
struct Running {
    tenant: TenantScope,
    host: String,
    action: AppLifecycleAction,
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
fn status(id: String, op: &Operation) -> AppOperationStatus {
    let p = op.snapshot();
    AppOperationStatus {
        operation_id: TaskId::from(id),
        status: p.status,
        phase: match p.phase {
            Phase::Queued => AppOperationPhase::Queued,
            Phase::Preflight => AppOperationPhase::Preflight,
            Phase::Executing => AppOperationPhase::Executing,
            Phase::Postflight => AppOperationPhase::Postflight,
            Phase::Terminal => AppOperationPhase::Terminal,
        },
    }
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/actions/preview", post(preview))
        .route("/agent-apps/actions", post(submit))
        .route("/agent-apps/operations/{id}", get(read).delete(cancel))
        .route("/agent-apps/operations/{id}/events", get(events))
        .layer(DefaultBodyLimit::max(8192))
        .layer(Extension(State { policy, store: Arc::new(Mutex::new(Store::default())) }))
}
async fn authorize(
    state: &State,
    headers: HeaderMap,
    tenant: TenantScope,
    host: String,
    action: AppLifecycleAction,
) -> Result<access::Host, StatusCode> {
    let policy = state.policy.clone();
    tokio::task::spawn_blocking(move || {
        access::authorize_action(policy.as_deref(), &headers, &tenant, &host, Some(action))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
fn environment(
    host: access::Host,
) -> Result<(LocalReadEnvironment, local::WriteEnvironment), StatusCode> {
    let write = host.lifecycle.ok_or(StatusCode::FORBIDDEN)?;
    Ok((
        LocalReadEnvironment {
            host_id: host.id,
            home: host.home,
            search_paths: host.search_paths,
            applications: host.applications,
        },
        local::WriteEnvironment { prefix: write.prefix, npm: write.npm },
    ))
}
async fn preview(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Json(request): Json<AppActionPreviewRequest>,
) -> Result<Json<AppActionPreview>, StatusCode> {
    let host =
        authorize(&state, headers, tenant.clone(), target.target_host_id.clone(), request.action)
            .await?;
    let (env, write) = environment(host)?;
    let write_copy = write.clone();
    let plan = tokio::task::spawn_blocking(move || local::preview(request, &env, &write_copy))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let id = uuid::Uuid::new_v4().to_string();
    let response = AppActionPreview {
        preview_id: AppActionPreviewId(id.clone()),
        request: plan.request.clone(),
        source: plan.source,
        confirmation_required: true,
        summary: "Review the selected application action in the administrator-approved prefix"
            .into(),
        display_command: None,
    };
    let mut store = state.store.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    store.previews.retain(|_, p| p.created.elapsed() < Duration::from_secs(300));
    if store.previews.len() >= 1024 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    store.previews.insert(
        id,
        Preview { tenant, host: target.target_host_id, created: Instant::now(), plan, write },
    );
    Ok(Json(response))
}
async fn submit(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Json(request): Json<AppActionSubmitRequest>,
) -> Result<(StatusCode, Json<AppActionExecutionResult>), StatusCode> {
    let action = {
        let store = state.store.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let p = store
            .previews
            .get(&request.preview_id.0)
            .filter(|p| p.tenant == tenant && p.host == target.target_host_id)
            .ok_or(StatusCode::NOT_FOUND)?;
        p.plan.request.action
    };
    let host =
        authorize(&state, headers.clone(), tenant.clone(), target.target_host_id.clone(), action)
            .await?;
    let (env, write) = environment(host)?;
    if !request.confirmed {
        return Err(StatusCode::CONFLICT);
    }
    let mut store = state.store.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let p = store
        .previews
        .get(&request.preview_id.0)
        .filter(|p| p.tenant == tenant && p.host == target.target_host_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    if p.created.elapsed() >= Duration::from_secs(300) || p.write != write {
        return Err(StatusCode::CONFLICT);
    }
    // Bound retained operations; terminal handles are reclaimed before admission.
    if store.operations.len() >= 1024 {
        store.operations.retain(|_, r| r.operation.snapshot().phase != Phase::Terminal);
    }
    if store.operations.len() >= 1024 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let p = store.previews.remove(&request.preview_id.0).ok_or(StatusCode::NOT_FOUND)?;
    let operation = Operation::new();
    let policy = state.policy.clone();
    let t = tenant.clone();
    let h = target.target_host_id.clone();
    let app = p.plan.request.app_id;
    // Reauthorize after queueing as well, so revoked/changed grants cannot start writes.
    if action == AppLifecycleAction::Uninstall {
        operation
            .start_action(async move {
                let expected = write.clone();
                local::uninstall(p.plan, env, write, move || {
                    let fresh =
                        access::authorize_action(policy.as_deref(), &headers, &t, &h, Some(action))
                            .map_err(|_| "authorization revoked")?;
                    let (_, current) = environment(fresh).map_err(|_| "write grant revoked")?;
                    if current != expected {
                        return Err("write environment changed".into());
                    }
                    Ok(())
                })
                .await
            })
            .map_err(|_| StatusCode::CONFLICT)?;
    } else {
        operation
            .start(vec![app], move || {
                let fresh =
                    access::authorize_action(policy.as_deref(), &headers, &t, &h, Some(action))
                        .map_err(|_| "authorization revoked")?;
                let (_, current) = environment(fresh).map_err(|_| "write grant revoked")?;
                if current != write {
                    return Err("write environment changed".into());
                }
                local::run(p.plan, env, write)
            })
            .map_err(|_| StatusCode::CONFLICT)?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    store
        .operations
        .insert(id.clone(), Running { tenant, host: target.target_host_id, action, operation });
    Ok((
        StatusCode::ACCEPTED,
        Json(AppActionExecutionResult {
            operation_id: TaskId::from(id),
            status: TaskStatus::Queued,
        }),
    ))
}
async fn owned(
    state: &State,
    tenant: TenantScope,
    headers: HeaderMap,
    host: String,
    id: &str,
) -> Result<Operation, StatusCode> {
    let (action, operation) = {
        let store = state.store.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let r = store
            .operations
            .get(id)
            .filter(|r| r.tenant == tenant && r.host == host)
            .ok_or(StatusCode::NOT_FOUND)?;
        (r.action, r.operation.clone())
    };
    authorize(state, headers, tenant, host, action).await?;
    Ok(operation)
}
async fn read(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<AppOperationStatus>, StatusCode> {
    let op = owned(&state, tenant, headers, target.target_host_id, &id).await?;
    Ok(Json(status(id, &op)))
}
async fn events(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<AppOperationStatus>, StatusCode> {
    let op =
        owned(&state, tenant.clone(), headers.clone(), target.target_host_id.clone(), &id).await?;
    let mut receiver = op.subscribe();
    if op.snapshot().phase != Phase::Terminal {
        let _ = tokio::time::timeout(Duration::from_secs(25), receiver.changed()).await;
    }
    owned(&state, tenant, headers, target.target_host_id, &id).await?;
    // Long polling resumes from current durable-in-process state, never restarts writes.
    Ok(Json(status(id, &op)))
}
async fn cancel(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<AppOperationStatus>, StatusCode> {
    let op = owned(&state, tenant, headers, target.target_host_id, &id).await?;
    op.cancel_and_wait().await;
    Ok(Json(status(id, &op)))
}
