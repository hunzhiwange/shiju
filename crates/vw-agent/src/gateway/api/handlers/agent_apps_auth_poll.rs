//! Read the execution host's scoped authorization flow snapshot.

use super::{
    agent_apps_auth_read::PolicyFile,
    agent_apps_auth_start::{FlowError, Runtime, ScopeQuery, authorize},
};
use axum::{
    Extension, Json,
    extract::{
        Path, Query,
        rejection::{PathRejection, QueryRejection},
    },
    http::HeaderMap,
};
use std::{sync::Arc, time::Instant};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthFlow, AuthFlowId},
    id::TenantScope,
};

pub(super) async fn poll(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(runtime): Extension<Arc<Runtime>>,
    id: Result<Path<String>, PathRejection>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Json<AuthFlow>, FlowError> {
    let execution = authorize(policy, tenant, headers, query).await?;
    let Path(id) = id.map_err(|_| AuthErrorCode::InvalidInput)?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| AuthErrorCode::NotFound)?;
    let id = AuthFlowId(id.to_string());

    // A concurrent host commit can hold the flow lock while persisting the account.
    let snapshot = tokio::task::spawn_blocking(move || {
        runtime.flows.snapshot(execution.scope(), &id, Instant::now)
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;
    Ok(Json(snapshot))
}
