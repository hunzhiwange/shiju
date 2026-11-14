//! Fixed lifecycle endpoints; opaque IDs never confer scope or host authority.
use super::access;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
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
pub(in crate::gateway::api::handlers) fn status(id: String, op: &Operation) -> AppOperationStatus {
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
            // Lifecycle discovery is restricted to the managed installation prefix.
            search_paths: vec![write.prefix.join("bin")],
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
enum OwnedOperation {
    #[cfg(target_os = "macos")]
    Prompt(Arc<super::super::agent_app_prompt_sync::Job>),
    Mcp(Arc<super::super::mcp_management_resync::Job>),
    Lifecycle(Operation),
    #[cfg(target_os = "macos")]
    Usage(Arc<super::apps_usage_refresh::RefreshOperation>),
}

impl OwnedOperation {
    fn operation(&self) -> &Operation {
        match self {
            #[cfg(target_os = "macos")]
            Self::Prompt(entry) => &entry.operation,
            Self::Lifecycle(operation) => operation,
            Self::Mcp(entry) => &entry.operation,
            #[cfg(target_os = "macos")]
            Self::Usage(entry) => &entry.operation,
        }
    }

    fn response(&self, id: String) -> Response {
        match self {
            #[cfg(target_os = "macos")]
            Self::Prompt(entry) => entry.response(id),
            Self::Lifecycle(operation) => Json(status(id, operation)).into_response(),
            Self::Mcp(entry) => entry.response(id),
            #[cfg(target_os = "macos")]
            Self::Usage(entry) => entry.response(),
        }
    }
}

type McpJobs = Option<Extension<super::super::mcp_management_resync::Jobs>>;

#[cfg(target_os = "macos")]
type UsageService = Option<Extension<Arc<super::usage_refresh::UsageRefresh>>>;

#[allow(clippy::too_many_arguments)]
async fn resolve_operation(
    state: &State,
    tenant: TenantScope,
    headers: HeaderMap,
    host: &str,
    id: &str,
    #[cfg(target_os = "macos")] prompts: Option<Extension<super::super::agent_app_prompt_sync::Jobs>>,
    mcp: McpJobs,
    #[cfg(target_os = "macos")] usage: UsageService,
    #[cfg(target_os = "macos")] cancel: bool,
) -> Result<OwnedOperation, Response> {
    #[cfg(target_os = "macos")]
    if let Some(Extension(jobs)) = prompts
        && let Some(entry) = jobs.owned(id, &tenant, host, &headers, state.policy.as_deref()).map_err(IntoResponse::into_response)? {
        return Ok(OwnedOperation::Prompt(entry));
    }
    if let Some(Extension(jobs)) = mcp
        && let Some(entry) = jobs
            .owned(
                id,
                &tenant,
                host,
                &headers,
                &super::super::mcp_management_read::PolicyFile(state.policy.clone()),
            )
            .map_err(IntoResponse::into_response)?
    {
        return Ok(OwnedOperation::Mcp(entry));
    }
    #[cfg(target_os = "macos")]
    if let Some(Extension(service)) = usage
        && let Some(entry) = service.operations.get(id)
    {
        entry
            .authorize(tenant, host, headers, service, cancel)
            .await
            .map_err(super::usage_read::ReadError::operation_response)?;
        return Ok(OwnedOperation::Usage(entry));
    }
    owned(state, tenant, headers, host.to_owned(), id)
        .await
        .map(OwnedOperation::Lifecycle)
        .map_err(IntoResponse::into_response)
}

async fn read(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
    #[cfg(target_os = "macos")] prompts: Option<Extension<super::super::agent_app_prompt_sync::Jobs>>,
    mcp: McpJobs,
    #[cfg(target_os = "macos")] usage: UsageService,
) -> Result<Response, Response> {
    let entry = resolve_operation(
        &state,
        tenant,
        headers,
        &target.target_host_id,
        &id,
        #[cfg(target_os = "macos")]
        prompts.clone(),
        mcp.clone(),
        #[cfg(target_os = "macos")]
        usage,
        #[cfg(target_os = "macos")]
        false,
    )
    .await?;
    Ok(entry.response(id))
}
async fn events(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
    #[cfg(target_os = "macos")] prompts: Option<Extension<super::super::agent_app_prompt_sync::Jobs>>,
    mcp: McpJobs,
    #[cfg(target_os = "macos")] usage: UsageService,
) -> Result<Response, Response> {
    let entry = resolve_operation(
        &state,
        tenant.clone(),
        headers.clone(),
        &target.target_host_id,
        &id,
        #[cfg(target_os = "macos")]
        prompts.clone(),
        mcp.clone(),
        #[cfg(target_os = "macos")]
        usage.clone(),
        #[cfg(target_os = "macos")]
        false,
    )
    .await?;
    let op = entry.operation();
    let mut receiver = op.subscribe();
    if op.snapshot().phase != Phase::Terminal {
        let _ = tokio::time::timeout(Duration::from_secs(25), receiver.changed()).await;
    }
    // Long polling resumes from current durable-in-process state, never restarts writes.
    let entry = resolve_operation(
        &state,
        tenant,
        headers,
        &target.target_host_id,
        &id,
        #[cfg(target_os = "macos")]
        prompts.clone(),
        mcp.clone(),
        #[cfg(target_os = "macos")]
        usage,
        #[cfg(target_os = "macos")]
        false,
    )
    .await?;
    Ok(entry.response(id))
}
async fn cancel(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
    #[cfg(target_os = "macos")] prompts: Option<Extension<super::super::agent_app_prompt_sync::Jobs>>,
    mcp: McpJobs,
    #[cfg(target_os = "macos")] usage: UsageService,
) -> Result<Response, Response> {
    let entry = resolve_operation(
        &state,
        tenant.clone(),
        headers.clone(),
        &target.target_host_id,
        &id,
        #[cfg(target_os = "macos")]
        prompts.clone(),
        mcp.clone(),
        #[cfg(target_os = "macos")]
        usage.clone(),
        #[cfg(target_os = "macos")]
        true,
    )
    .await?;
    entry.operation().cancel_and_wait().await;
    let entry = match entry {
        OwnedOperation::Lifecycle(_) => entry,
        _ => {
            resolve_operation(
                &state,
                tenant,
                headers,
                &target.target_host_id,
                &id,
                #[cfg(target_os = "macos")]
                prompts,
                mcp,
                #[cfg(target_os = "macos")]
                usage,
                #[cfg(target_os = "macos")]
                true,
            )
            .await?
        }
    };
    Ok(entry.response(id))
}
