//! Restricted archived-root deletion; partial failures preserve the root for retry.
use std::path::PathBuf;
use axum::{Json, extract::{Path, Query, State}, http::HeaderMap};
use vw_api_transport::id::TenantScope;
use vw_api_session::session::ArchivedChatDeleteResponse;
use gateway_types::ApiError;
use gateway_instance::host::InstanceQuery;
use agent_project::instance;
use session_port::SessionScope;
use session_info::Info;
use session_engine::message;
use crate::session_host::SessionHost;
use crate::shared::{has_explicit_directory, lifecycle_api_error};
use crate::archived_delete::delete_order;
async fn candidates<H: SessionHost>(tenant: &TenantScope) -> Result<Vec<Info>, ApiError> {
    let tenant = tenant.clone();
    agent_database::run_isolated(move || H::deletion_candidates(&tenant))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .map_err(|error| ApiError::internal(error.to_string()))
}

fn root<'a>(id: &str, records: &'a [Info]) -> Result<&'a Info, ApiError> {
    let mut matches = records.iter().filter(|info| info.id == id);
    let root = matches.next().ok_or_else(|| ApiError::not_found("archived session not found"))?;
    if matches.next().is_some() {
        return Err(ApiError::bad_request("ambiguous session ID across projects"));
    }
    Ok(root)
}

fn canonical_directory(directory: &str) -> PathBuf {
    std::fs::canonicalize(directory).unwrap_or_else(|_| PathBuf::from(directory))
}

pub async fn delete<H: SessionHost>(
    State(state): State<H::State>,
    Path(id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<ArchivedChatDeleteResponse>, ApiError> {
    let tenant = H::current_tenant_scope();
    let initial = candidates::<H>(&tenant).await?;
    let initial_root = root(&id, &initial)?;
    let directory = if has_explicit_directory(&query, &headers) {
        H::resolve_directory(&query, &headers)
    } else {
        initial_root.directory.clone()
    };
    let scope = SessionScope::new(tenant.clone(), directory).map_err(lifecycle_api_error)?;
    if canonical_directory(&initial_root.directory) != scope.directory() {
        return Err(ApiError::not_found("archived session does not belong to this directory"));
    }
    let order = delete_order(initial_root, &initial)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    H::with_tenant_instance(
        tenant.clone(),
        scope.directory().to_string_lossy().into_owned(),
        move || {
            Box::pin(async move {
                // Writers holding history always acquire metadata second.
                let mut lock_ids = order.clone();
                lock_ids.sort();
                let mut history = Vec::with_capacity(lock_ids.len());
                for id in &lock_ids {
                    history.push(message::lock_history(id).await);
                }
                let _metadata = H::metadata_lock(&tenant).await;
                let current = candidates::<H>(&tenant).await?;
                let current_root = root(&id, &current)?;
                let project = instance::project()
                    .ok_or_else(|| ApiError::bad_request("no active project context"))?;
                if current_root.project_id != project.id
                    || canonical_directory(&current_root.directory) != scope.directory()
                {
                    return Err(ApiError::not_found("archived session scope changed"));
                }
                let current_order = delete_order(current_root, &current)
                    .map_err(|error| ApiError::bad_request(error.to_string()))?;
                if current_order != order {
                    return Err(ApiError::bad_request(
                        "archived session tree changed; refresh and retry",
                    ));
                }
                let mut deleted_ids = Vec::with_capacity(order.len());
                for id in order {
                    let info = current
                        .iter()
                        .find(|info| info.id == id)
                        .expect("validated deletion candidate")
                        .clone();
                    let result = H::remove_archived_locked(info).await;
                    // Also invalidate a failed node whose history may have been partially cleaned.
                    H::invalidate_engine(&state, &id).await;
                    result.map_err(|error| ApiError::internal(error.to_string()))?;
                    deleted_ids.push(id);
                }
                Ok(Json(ArchivedChatDeleteResponse { deleted_ids }))
            })
        },
    )
    .await
}
