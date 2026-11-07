//! Scoped provider work on the existing host operation executor.
use super::*;
use axum::{extract::Query, http::HeaderMap, routing::post};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use vw_agent_apps::providers::operations::ProviderOperation;
use vw_api_types::{app_providers::operations::*, id::TaskId, task::TaskStatus};
#[derive(Clone)]
struct State {
    policy: Option<PathBuf>,
    operations: Arc<Mutex<HashMap<String, Running>>>,
}
struct Running {
    tenant: TenantScope,
    request: ProviderOperationRequest,
    operation: ProviderOperation,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/app-providers/operations", post(submit))
        .route("/app-providers/operations/{id}", get(read).delete(cancel))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(State { policy, operations: Arc::new(Mutex::new(HashMap::new())) }))
}
async fn authorize(
    state: &State,
    headers: HeaderMap,
    tenant: TenantScope,
    host: String,
    read: bool,
) -> Result<(), StatusCode> {
    let policy = state.policy.clone();
    tokio::task::spawn_blocking(move || {
        super::super::agent_apps::provider_editor_context(
            policy.as_deref(),
            &headers,
            &tenant,
            &host,
            true,
        )?;
        if read {
            super::super::agent_apps::provider_editor_context(
                policy.as_deref(),
                &headers,
                &tenant,
                &host,
                false,
            )?;
        }
        Ok(())
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn submit(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    body: Result<Json<ProviderOperationRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProviderOperationProgress>), StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    vw_agent_apps::providers::validate_identity(&request.request_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    match &request.action {
        ProviderOperationAction::Project { id, apps } => {
            vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
            vw_agent_apps::providers::universal_sync::validate_request(
                &vw_api_types::app_providers::universal_sync::UniversalSyncRequest {
                    apps: apps.clone(),
                },
            )
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        }
        ProviderOperationAction::Import { app: AppId::Dsh | AppId::ClaudeDesktop, .. } => {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        _ => {}
    }
    let needs_read = matches!(request.action, ProviderOperationAction::Import { .. });
    authorize(&state, headers.clone(), tenant.clone(), request.target_host_id.clone(), needs_read)
        .await?;
    let mut registry = state.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if let Some(r) = registry.values().find(|r| {
        r.tenant == tenant
            && r.request.target_host_id == request.target_host_id
            && r.request.request_id == request.request_id
    }) {
        if serde_json::to_value(&r.request).ok() != serde_json::to_value(&request).ok() {
            return Err(StatusCode::CONFLICT);
        }
        return Ok((StatusCode::ACCEPTED, Json(r.operation.snapshot())));
    }
    if registry.len() >= 1024 {
        registry.retain(|_, r| {
            matches!(r.operation.snapshot().status, TaskStatus::Queued | TaskStatus::Running)
        });
    }
    if registry.len() >= 1024 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let operation = ProviderOperation::new(TaskId::from(id.clone()), request.request_id.clone());
    let policy = state.policy.clone();
    let scoped = tenant.clone();
    let input = request.clone();
    operation
        .start(move || {
            let (home, _) = super::super::agent_apps::provider_editor_context(
                policy.as_deref(),
                &headers,
                &scoped,
                &input.target_host_id,
                true,
            )
            .map_err(|_| "authorization unavailable")?;
            if needs_read {
                super::super::agent_apps::provider_editor_context(
                    policy.as_deref(),
                    &headers,
                    &scoped,
                    &input.target_host_id,
                    false,
                )
                .map_err(|_| "authorization unavailable")?;
            }
            let store = store(database)?;
            match input.action {
                ProviderOperationAction::Project { id, apps } => {
                    use vw_api_types::app_providers::universal_sync::{
                        UniversalCatalogResult, UniversalSyncRequest,
                    };
                    let results =
                        store.universal_sync(&scoped, &id, &UniversalSyncRequest { apps })?;
                    if results
                        .iter()
                        .any(|r| matches!(r.catalog, UniversalCatalogResult::Failed { .. }))
                    {
                        return Err("provider projection incomplete".into());
                    }
                    Ok(())
                }
                ProviderOperationAction::Import { app, trigger } => {
                    #[cfg(target_os = "macos")]
                    {
                        store.import_native(&scoped, &home, app, trigger).map(|_| ())
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = (store, home, app, trigger);
                        Err("provider import unsupported".into())
                    }
                }
            }
        })
        .map_err(|_| StatusCode::CONFLICT)?;
    let progress = operation.snapshot();
    registry.insert(id, Running { tenant, request, operation });
    Ok((StatusCode::ACCEPTED, Json(progress)))
}
async fn owned(
    state: &State,
    tenant: TenantScope,
    headers: HeaderMap,
    host: String,
    id: &str,
) -> Result<ProviderOperation, StatusCode> {
    authorize(state, headers, tenant.clone(), host.clone(), false).await?;
    let registry = state.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    registry
        .get(id)
        .filter(|r| r.tenant == tenant && r.request.target_host_id == host)
        .map(|r| r.operation.clone())
        .ok_or(StatusCode::NOT_FOUND)
}
async fn read(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<ProviderOperationProgress>, StatusCode> {
    Ok(Json(owned(&state, tenant, headers, target.target_host_id, &id).await?.snapshot()))
}
async fn cancel(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<ProviderOperationProgress>, StatusCode> {
    Ok(Json(owned(&state, tenant, headers, target.target_host_id, &id).await?.cancel()))
}
