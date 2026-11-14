//! Execution API boundary. Native observations and writes must be supplied by
//! their domain owners before captured compositions can be previewed or applied.
use super::*;
use axum::{extract::Path, routing::{get, post}};
use vw_api_types::project_profiles::ProjectProfileId;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    project_id: ProjectId,
    target_host_id: String,
    scope: ProjectProfileScope,
    expected_version: u64,
}

enum Action {
    Preview(ProjectProfileId, u64),
    Apply(ProjectProfileId, u64),
    Result,
    Operation(String),
    Cancel(String),
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/project-profiles/{id}/preview", post(preview))
        .route("/agent-apps/project-profiles/{id}/apply", post(apply))
        .route("/agent-apps/project-profiles/current/result", get(result))
        .route("/agent-apps/project-profiles/operations/{operation_id}", get(operation))
        .route("/agent-apps/project-profiles/operations/{operation_id}/cancel", post(cancel))
}

macro_rules! profile_handler {
    ($name:ident, $action:ident) => {
        async fn $name(
            Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
            database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Path(id): Path<String>,
            body: Result<Json<Request>, JsonRejection>,
        ) -> Response {
            let Ok(Json(body)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
            run(policy, tenant, database, headers,
                Context { project_id: body.project_id, target_host_id: body.target_host_id, scope: body.scope },
                Action::$action(ProjectProfileId(id), body.expected_version)).await
        }
    };
}
profile_handler!(preview, Preview);
profile_handler!(apply, Apply);

async fn result(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    query: Result<Query<Context>, QueryRejection>,
) -> Response {
    let Ok(Query(context)) = query else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    run(policy, tenant, database, headers, context, Action::Result).await
}
async fn operation(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Path(id): Path<String>,
    query: Result<Query<Context>, QueryRejection>,
) -> Response {
    let Ok(Query(context)) = query else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    run(policy, tenant, database, headers, context, Action::Operation(id)).await
}
async fn cancel(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Path(id): Path<String>,
    body: Result<Json<Context>, JsonRejection>,
) -> Response {
    let Ok(Json(context)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    run(policy, tenant, database, headers, context, Action::Cancel(id)).await
}

async fn run(
    policy: Policy, tenant: TenantScope, database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, context: Context, action: Action,
) -> Response {
    let outcome = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, Failure> {
        let project = ProjectScope::new(tenant, context.project_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_project"))?;
        vw_agent_apps::providers::validate_identity(&context.target_host_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_host"))?;
        // Reload policy on every request, including result and cancellation. A
        // previously authorized operation ID can never act as a capability.
        let executor = if matches!(&action, Action::Apply(..) | Action::Cancel(_)) {
            super::super::agent_apps::authorize_project_profiles_maintenance(
                policy.0.as_deref(), &headers, &project, &context.target_host_id)
        } else {
            super::super::agent_apps::authorize_project_profiles(
                policy.0.as_deref(), &headers, &project, &context.target_host_id, false)
        }.map_err(|status| (status, "profile_access_denied"))?;
        if let Action::Operation(id) | Action::Cancel(id) = &action {
            if id.trim().is_empty() { return Err((StatusCode::BAD_REQUEST, "invalid_operation")); }
            // No operation is admitted until native execution is wired. Never
            // adopt another subsystem's operation IDs or acknowledge fake cancel.
            return Err((StatusCode::NOT_FOUND, "profile_operation_not_found"));
        }
        let store = ProjectProfileStore::new(
            database.ok_or((StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?.0, executor.clone(),
        ).map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?;
        store.with_operation(project.tenant_scope(), |op| {
            let scope = vw_agent_apps::project_profiles::snapshot::SnapshotScope {
                project: &project, target_host_id: &executor, group: context.scope,
            };
            // Verify physical slot ownership even for read-only requests.
            op.current(&project, &executor, context.scope)?;
            match &action {
                Action::Result => Ok(Ok(serde_json::json!({"result": op.application_result(&scope)?}))),
                Action::Preview(id, version) | Action::Apply(id, version) => {
                    if id.0.trim().is_empty() { return Ok(Err((StatusCode::BAD_REQUEST, "invalid_profile"))); }
                    let Some(profile) = op.get(project.tenant_scope(), id)? else {
                        return Ok(Err((StatusCode::NOT_FOUND, "profile_not_found")));
                    };
                    if profile.version != *version { return Ok(Err((StatusCode::CONFLICT, "profile_version_conflict"))); }
                    // No default PreviewState: an empty observation falsely
                    // classifies valid references as missing. No selected write.
                    Ok(Err((StatusCode::SERVICE_UNAVAILABLE, if matches!(&action, Action::Preview(..)) {
                        "profile_observation_unavailable"
                    } else { "profile_execution_unavailable" })))
                }
                Action::Operation(_) | Action::Cancel(_) => unreachable!(),
            }
        }).map_err(|_| (StatusCode::CONFLICT, "profile_state_invalid"))?
    }).await;
    match outcome {
        Ok(Ok(value)) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => failure((StatusCode::SERVICE_UNAVAILABLE, "executor_unavailable")),
    }
}
