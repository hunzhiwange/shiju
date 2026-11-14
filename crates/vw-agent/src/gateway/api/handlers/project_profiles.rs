//! Tenant-shared project compositions; executor identity comes from host policy.
use axum::{Extension, Json, Router, extract::{Query, DefaultBodyLimit, rejection::{QueryRejection, JsonRejection}}, http::{HeaderMap, StatusCode, header}, response::{IntoResponse, Response}, routing::get};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{ProjectId, ProjectScope, TenantScope, project_profiles::ProjectProfileScope};
use crate::{global_database::GlobalDatabase, gateway::agent_apps::profile_store::ProjectProfileStore};

#[path = "project_profiles_maintenance.rs"]
mod maintenance;

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    project_id: ProjectId,
    target_host_id: String,
    scope: ProjectProfileScope,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateRequest {
    project_id: ProjectId,
    target_host_id: String,
    scope: ProjectProfileScope,
    name: String,
}
#[derive(Clone, Copy)]
enum Operation { List, Current, Create }
type Failure = (StatusCode, &'static str);

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/project-profiles", get(list).post(create))
        .route("/agent-apps/project-profiles/current", get(current))
        .merge(maintenance::router())
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
async fn list(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, query: Result<Query<Context>, QueryRejection>) -> Response {
    dispatch(policy, tenant, database, headers, query.map(|Query(q)| q).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request")), Operation::List, None).await
}
async fn current(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, query: Result<Query<Context>, QueryRejection>) -> Response {
    dispatch(policy, tenant, database, headers, query.map(|Query(q)| q).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_request")), Operation::Current, None).await
}
async fn create(Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, body: Result<Json<CreateRequest>, JsonRejection>) -> Response {
    let Ok(Json(body)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    if body.name.trim().is_empty() { return failure((StatusCode::BAD_REQUEST, "empty_name")); }
    dispatch(policy, tenant, database, headers, Ok(Context { project_id: body.project_id, target_host_id: body.target_host_id, scope: body.scope }), Operation::Create, Some(body.name)).await
}
async fn dispatch(policy: Policy, tenant: TenantScope, database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, context: Result<Context, Failure>, operation: Operation, name: Option<String>) -> Response {
    let context = match context { Ok(c) => c, Err(e) => return failure(e) };
    match tokio::task::spawn_blocking(move || execute(policy, tenant, database.map(|Extension(db)| db), headers, context, operation, name)).await {
        Ok(Ok(value)) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
        Ok(Err(e)) => failure(e),
        Err(_) => failure((StatusCode::SERVICE_UNAVAILABLE, "executor_unavailable")),
    }
}
fn failure((status, code): Failure) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], Json(serde_json::json!({"error":{"code":code,"message":code}}))).into_response()
}
fn execute(policy: Policy, tenant: TenantScope, database: Option<GlobalDatabase>, headers: HeaderMap, context: Context, operation: Operation, name: Option<String>) -> Result<serde_json::Value, Failure> {
    let project = ProjectScope::new(tenant, context.project_id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_project"))?;
    vw_agent_apps::providers::validate_identity(&context.target_host_id).map_err(|_| (StatusCode::BAD_REQUEST, "invalid_host"))?;
    let executor = super::agent_apps::authorize_project_profiles(policy.0.as_deref(), &headers, &project, &context.target_host_id, matches!(operation, Operation::Create))
        .map_err(|s| (s, "profile_access_denied"))?;
    let store = ProjectProfileStore::new(database.ok_or((StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?, executor.clone())
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?;
    store.with_operation(project.tenant_scope(), |op| {
        let value = match operation {
            Operation::List => serde_json::to_value(op.list(project.tenant_scope())?)?,
            Operation::Current => serde_json::json!({"profileId": op.current(&project, &executor, context.scope)?}),
            Operation::Create => {
                // Fail closed until effective native-state readers are supplied by
                // domain owners. Never substitute catalog intent or empty payloads.
                use vw_agent_apps::project_profiles::snapshot::{SnapshotReadFailure, SnapshotScope};
                let scope = SnapshotScope { project: &project, target_host_id: &executor, group: context.scope };
                let profile = op.create_from_current(name.as_deref().unwrap_or_default(), &scope,
                    |_| Err(SnapshotReadFailure::Unavailable),
                    |_| Err(SnapshotReadFailure::Unavailable),
                    |_| Err(SnapshotReadFailure::Unavailable),
                    |_| Err(SnapshotReadFailure::Unavailable))?;
                serde_json::to_value(profile)?
            }
        };
        Ok(value)
    }).map_err(|_| if matches!(operation, Operation::Create) { (StatusCode::SERVICE_UNAVAILABLE, "snapshot_unavailable") } else { (StatusCode::CONFLICT, "profile_state_invalid") })
}
