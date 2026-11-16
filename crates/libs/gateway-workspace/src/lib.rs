//! Workspace routes with host authorization and operation status injected.
#![cfg(target_os = "macos")]
use axum::http::{HeaderMap, StatusCode};
use std::path::{Path, PathBuf};
use vw_api_transport::ProjectScope;
use app_lifecycle::operations::Operation;
use vw_api_app_operations::AppOperationStatus;

/// Trusted executor fields used by native workspace services.
pub struct WorkspaceHost {
    pub id: String,
    pub home: PathBuf,
}

pub type AuthorizeWorkspace = fn(Option<&Path>, &HeaderMap, &ProjectScope, &str, bool)
    -> Result<(WorkspaceHost, String), StatusCode>;
pub type OperationStatus = fn(String, &Operation) -> AppOperationStatus;

mod workspace_directory;
mod workspace_memory;
pub use workspace_directory::router as directory_router;
pub use workspace_memory::router as memory_router;

pub fn authorize_workspace(
    policy: Option<&std::path::Path>, headers: &axum::http::HeaderMap,
    project: &vw_api_transport::ProjectScope, target: &str, write: bool,
) -> Result<(WorkspaceHost, String), axum::http::StatusCode> {
    app_access_service::access::authorize_workspace(policy, headers, project, target, write)
        .map(|(host, actor)| (WorkspaceHost { id: host.id, home: host.home }, actor))
}

#[cfg(test)]
mod host_migration_17_tests;
