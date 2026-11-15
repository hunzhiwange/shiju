//! Restricted archived-root deletion; partial failures preserve the root for retry.

use std::path::PathBuf;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use vw_api_transport::id::TenantScope;
use vw_api_session::session::ArchivedChatDeleteResponse;

use super::shared::{has_explicit_directory, lifecycle_api_error, with_tenant_instance};
use crate::gateway::chat::invalidate_session_query_engine;
use crate::gateway::instance::{InstanceQuery, resolve_directory};
use crate::gateway::{ApiError, AppState};
use crate::project::instance;
use crate::session::runtime::SessionScope;
use crate::session::session::Info;
use crate::session::{message, session, ui_store};

async fn candidates(tenant: &TenantScope) -> Result<Vec<Info>, ApiError> {
    let tenant = tenant.clone();
    crate::global_database::run_isolated(move || ui_store::load_agent_deletion_candidates(&tenant))
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

pub(super) async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<ArchivedChatDeleteResponse>, ApiError> {
    let tenant = ui_store::current_tenant_scope();
    let initial = candidates(&tenant).await?;
    let initial_root = root(&id, &initial)?;
    let directory = if has_explicit_directory(&query, &headers) {
        resolve_directory(&query, &headers)
    } else {
        initial_root.directory.clone()
    };
    let scope = SessionScope::new(tenant.clone(), directory).map_err(lifecycle_api_error)?;
    if canonical_directory(&initial_root.directory) != scope.directory() {
        return Err(ApiError::not_found("archived session does not belong to this directory"));
    }
    let order = delete_order(initial_root, &initial)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    with_tenant_instance(
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
                let _metadata = session::lock_metadata_writes(&tenant).await;
                let current = candidates(&tenant).await?;
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
                    let result = session::remove_archived_locked(info).await;
                    // Also invalidate a failed node whose history may have been partially cleaned.
                    invalidate_session_query_engine(&state, &id).await;
                    result.map_err(|error| ApiError::internal(error.to_string()))?;
                    deleted_ids.push(id);
                }
                Ok(Json(ArchivedChatDeleteResponse { deleted_ids }))
            })
        },
    )
    .await
}

pub(super) use gateway_session::archived_delete::delete_order;
