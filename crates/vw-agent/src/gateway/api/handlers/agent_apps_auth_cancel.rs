//! Confirm local cancellation of an execution host's scoped authorization flow.

use super::{
    agent_apps_auth_read::PolicyFile,
    agent_apps_auth_start::{FlowError, Runtime, ScopeQuery, authorize},
};
use axum::{
    Extension,
    extract::{
        Path, Query,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
};
use std::{sync::Arc, time::Instant};
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthFlowId},
    id::TenantScope,
};

pub(super) async fn cancel(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(runtime): Extension<Arc<Runtime>>,
    id: Result<Path<String>, PathRejection>,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<StatusCode, FlowError> {
    let execution = authorize(policy, tenant, headers, query).await?;
    let Path(id) = id.map_err(|_| AuthErrorCode::InvalidInput)?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| AuthErrorCode::NotFound)?;
    let id = AuthFlowId(id.to_string());

    #[cfg(target_os = "macos")]
    let worker = tokio::runtime::Handle::current();
    // The worker finishes even if the HTTP client stops waiting for confirmation.
    tokio::task::spawn_blocking(move || {
        let scope = execution.scope();
        let result = runtime.flows.cancel(scope, &id, Instant::now);
        #[cfg(target_os = "macos")]
        if result == Err(AuthErrorCode::ExecutionHostUnavailable)
            && scope.service == vw_api_types::agent_apps_auth::AuthService::ClaudeOfficial
        {
            // Wait for the async CLI supervisor outside the flow's cleanup lock.
            worker.block_on(runtime.wait_for_claude_cleanup(&id))?;
            return runtime.flows.cancel(scope, &id, Instant::now);
        }
        result
    })
    .await
    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)??;
    Ok(StatusCode::NO_CONTENT)
}
