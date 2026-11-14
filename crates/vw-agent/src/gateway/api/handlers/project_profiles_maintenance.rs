//! Narrow maintenance endpoints; no caller-controlled command or snapshot payload.
use super::*;
use axum::{extract::Path, routing::{delete, post}};
use vw_api_types::project_profiles::ProjectProfileId;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Mutation {
    project_id: ProjectId,
    target_host_id: String,
    scope: ProjectProfileScope,
    expected_version: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Rename {
    project_id: ProjectId,
    target_host_id: String,
    scope: ProjectProfileScope,
    expected_version: u64,
    name: String,
}

enum Maintenance {
    Rename(ProjectProfileId, u64, String),
    Resnapshot(ProjectProfileId, u64),
    Delete(ProjectProfileId, u64),
    Clear,
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/project-profiles/{id}", delete(remove).patch(rename))
        .route("/agent-apps/project-profiles/{id}/resnapshot", post(resnapshot))
        .route("/agent-apps/project-profiles/current/clear", post(clear))
}

async fn rename(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Path(id): Path<String>,
    body: Result<Json<Rename>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    if body.name.trim().is_empty() { return failure((StatusCode::BAD_REQUEST, "empty_name")); }
    run(policy, tenant, database, headers,
        Context { project_id: body.project_id, target_host_id: body.target_host_id, scope: body.scope },
        Maintenance::Rename(ProjectProfileId(id), body.expected_version, body.name)).await
}

async fn remove(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Path(id): Path<String>,
    body: Result<Json<Mutation>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    run(policy, tenant, database, headers, Context { project_id: body.project_id, target_host_id: body.target_host_id, scope: body.scope },
        Maintenance::Delete(ProjectProfileId(id), body.expected_version)).await
}

async fn resnapshot(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Path(id): Path<String>,
    body: Result<Json<Mutation>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    run(policy, tenant, database, headers, Context { project_id: body.project_id, target_host_id: body.target_host_id, scope: body.scope },
        Maintenance::Resnapshot(ProjectProfileId(id), body.expected_version)).await
}

async fn clear(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, headers: HeaderMap,
    body: Result<Json<Context>, JsonRejection>,
) -> Response {
    let Ok(Json(context)) = body else { return failure((StatusCode::BAD_REQUEST, "invalid_request")); };
    run(policy, tenant, database, headers, context, Maintenance::Clear).await
}

async fn run(
    policy: Policy, tenant: TenantScope, database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, context: Context, action: Maintenance,
) -> Response {
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, Failure> {
        let project = ProjectScope::new(tenant, context.project_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_project"))?;
        vw_agent_apps::providers::validate_identity(&context.target_host_id)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_host"))?;
        let executor = super::super::agent_apps::authorize_project_profiles_maintenance(
            policy.0.as_deref(), &headers, &project, &context.target_host_id,
        ).map_err(|status| (status, "profile_access_denied"))?;
        let store = ProjectProfileStore::new(
            database.ok_or((StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?.0,
            executor.clone(),
        ).map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"))?;
        store.with_operation(project.tenant_scope(), |op| {
            // Target lookup and version validation precede every entity mutation.
            let (id, version) = match &action {
                Maintenance::Rename(id, version, _) | Maintenance::Resnapshot(id, version)
                | Maintenance::Delete(id, version) => (Some(id), Some(*version)),
                Maintenance::Clear => (None, None),
            };
            if let Some(id) = id {
                anyhow::ensure!(!id.0.trim().is_empty(), "invalid profile id");
                let profile = op.get(project.tenant_scope(), id)?
                    .ok_or_else(|| anyhow::anyhow!("project profile not found"))?;
                anyhow::ensure!(Some(profile.version) == version, "profile version conflict");
            }
            match action {
                Maintenance::Rename(id, version, name) => {
                    op.rename(project.tenant_scope(), &id, &name, version)?;
                    Ok(serde_json::to_value(op.get(project.tenant_scope(), &id)?)?)
                }
                Maintenance::Delete(id, version) => {
                    op.delete(project.tenant_scope(), &id, version)?;
                    Ok(serde_json::json!({"deleted": op.get(project.tenant_scope(), &id)?.is_none()}))
                }
                Maintenance::Clear => {
                    op.clear(&project, &executor, context.scope)?;
                    Ok(serde_json::json!({"profileId": op.current(&project, &executor, context.scope)?}))
                }
                // As with creation, native effective-state readers are not wired.
                // Reject before any write rather than accepting supplied snapshots.
                Maintenance::Resnapshot(_, _) => anyhow::bail!("snapshot_unavailable"),
            }
        }).map_err(|error| {
            if error.to_string() == "snapshot_unavailable" {
                (StatusCode::SERVICE_UNAVAILABLE, "snapshot_unavailable")
            } else {
                (StatusCode::CONFLICT, "profile_state_invalid")
            }
        })
    }).await;
    match result {
        Ok(Ok(value)) => ([(header::CACHE_CONTROL, "no-store")], Json(value)).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => failure((StatusCode::SERVICE_UNAVAILABLE, "executor_unavailable")),
    }
}
