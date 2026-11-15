//! Host project/configuration adapters for the worktree routes.
use super::project::{load_project, project_context_directory, project_scope};
use gateway_types::ApiError;
use gateway_worktree::{HostFuture, WorktreeHost};
use std::path::PathBuf;
use vw_api_transport::id::{ProjectScope, TenantScope};

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    crate::worktree::initialize_host();
    gateway_worktree::router(WorktreeHost { context, settings })
}

fn context(tenant: TenantScope, project: String) -> HostFuture<'static, (ProjectScope, PathBuf)> {
    Box::pin(async move {
        let scope = project_scope(tenant, project)?;
        let info = load_project(&scope).await?;
        Ok((scope, project_context_directory(&info)?.into()))
    })
}

fn settings(scope: &ProjectScope) -> HostFuture<'_, config_worktree::WorktreeSettings> {
    Box::pin(async move {
        Ok(crate::config::get_global_for_tenant(scope.tenant_scope())
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?
            .worktree_settings)
    })
}

// These integration tests exercise the host project router and persisted fixtures.
#[cfg(test)]
#[path = "worktree_management_tests.rs"]
mod tests;
