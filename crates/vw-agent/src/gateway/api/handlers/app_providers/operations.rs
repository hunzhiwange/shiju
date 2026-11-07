//! Scoped provider work on the existing host operation executor.
use super::{
    editor_merge::{self, Editor},
    *,
};
use axum::{
    extract::{OriginalUri, Query},
    http::HeaderMap,
    routing::post,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
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
    editor: Editor,
    operations: Arc<Mutex<HashMap<String, Running>>>,
}
struct Running {
    tenant: TenantScope,
    host: String,
    request_id: String,
    fingerprint: [u8; 32],
    operation: ProviderOperation,
    editor_outcome: Arc<Mutex<Option<ProviderEditorOutcome>>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
#[cfg(test)]
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    router_with_editor(policy.clone(), Editor::new(policy))
}
pub(super) fn router_with_editor(policy: Option<PathBuf>, editor: Editor) -> Router {
    Router::new()
        .route("/app-providers/operations", post(submit).layer(DefaultBodyLimit::max(4096)))
        .route(
            "/app-providers/operations/editor-save",
            post(submit).layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route("/app-providers/operations/{id}", get(read).delete(cancel))
        .layer(Extension(State {
            policy,
            editor,
            operations: Arc::new(Mutex::new(HashMap::new())),
        }))
}

impl Running {
    fn progress(&self, mut progress: ProviderOperationProgress) -> ProviderOperationProgress {
        if matches!(progress.status, TaskStatus::Completed | TaskStatus::Failed) {
            progress.editor_outcome =
                self.editor_outcome.lock().unwrap_or_else(|e| e.into_inner()).clone();
        }
        progress
    }
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
    OriginalUri(uri): OriginalUri,
    body: Result<Json<ProviderOperationRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProviderOperationProgress>), StatusCode> {
    let Json(request) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    if uri.path().ends_with("/operations/editor-save")
        != matches!(request.action, ProviderOperationAction::EditorSave { .. })
    {
        return Err(StatusCode::BAD_REQUEST);
    }
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
        ProviderOperationAction::EditorSave { app, id, input } => {
            if *app != AppId::Claude {
                return Err(StatusCode::NOT_IMPLEMENTED);
            }
            if input.target_host_id != request.target_host_id {
                return Err(StatusCode::BAD_REQUEST);
            }
            vw_agent_apps::providers::validate_identity(id).map_err(|_| StatusCode::BAD_REQUEST)?;
        }
        _ => {}
    }
    let encoded = serde_json::to_vec(&request).map_err(|_| StatusCode::BAD_REQUEST)?;
    let fingerprint = Sha256::digest(&encoded).into();
    let needs_read = matches!(request.action, ProviderOperationAction::Import { .. });
    authorize(&state, headers.clone(), tenant.clone(), request.target_host_id.clone(), needs_read)
        .await?;
    let mut registry = state.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if let Some(r) = registry.values().find(|r| {
        r.tenant == tenant && r.host == request.target_host_id && r.request_id == request.request_id
    }) {
        if r.fingerprint != fingerprint {
            return Err(StatusCode::CONFLICT);
        }
        return Ok((StatusCode::ACCEPTED, Json(r.progress(r.operation.snapshot()))));
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
    let editor = state.editor.clone();
    let editor_outcome = Arc::new(Mutex::new(None));
    let worker_outcome = Arc::clone(&editor_outcome);
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
            match input.action {
                ProviderOperationAction::EditorSave { app, id, input } => {
                    vw_agent_apps::providers::operations::planned(1);
                    let result =
                        editor_merge::save_sync(editor, scoped, database, headers, app, id, input);
                    let outcome = match result {
                        Ok(Json(result)) => {
                            vw_agent_apps::providers::operations::advanced();
                            ProviderEditorOutcome::Reviewed { result }
                        }
                        Err(status) => ProviderEditorOutcome::Rejected { status: status.as_u16() },
                    };
                    let succeeded = matches!(outcome, ProviderEditorOutcome::Reviewed { .. });
                    *worker_outcome.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
                    if succeeded { Ok(()) } else { Err("editor save rejected".into()) }
                }
                ProviderOperationAction::Project { id, apps } => {
                    use vw_api_types::app_providers::universal_sync::{
                        UniversalCatalogResult, UniversalSyncRequest,
                    };
                    let results = store(database)?.universal_sync(
                        &scoped,
                        &id,
                        &UniversalSyncRequest { apps },
                    )?;
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
                        store(database)?.import_native(&scoped, &home, app, trigger).map(|_| ())
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = (database, home, app, trigger);
                        Err("provider import unsupported".into())
                    }
                }
            }
        })
        .map_err(|_| StatusCode::CONFLICT)?;
    let running = Running {
        tenant,
        host: request.target_host_id,
        request_id: request.request_id,
        fingerprint,
        operation,
        editor_outcome,
    };
    let progress = running.progress(running.operation.snapshot());
    registry.insert(id, running);
    Ok((StatusCode::ACCEPTED, Json(progress)))
}
async fn owned(
    state: &State,
    tenant: TenantScope,
    headers: HeaderMap,
    host: String,
    id: &str,
    cancel: bool,
) -> Result<ProviderOperationProgress, StatusCode> {
    authorize(state, headers, tenant.clone(), host.clone(), false).await?;
    let registry = state.operations.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    registry
        .get(id)
        .filter(|r| r.tenant == tenant && r.host == host)
        .map(|r| r.progress(if cancel { r.operation.cancel() } else { r.operation.snapshot() }))
        .ok_or(StatusCode::NOT_FOUND)
}
async fn read(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<ProviderOperationProgress>, StatusCode> {
    Ok(Json(owned(&state, tenant, headers, target.target_host_id, &id, false).await?))
}
async fn cancel(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Path(id): Path<String>,
) -> Result<Json<ProviderOperationProgress>, StatusCode> {
    Ok(Json(owned(&state, tenant, headers, target.target_host_id, &id, true).await?))
}
