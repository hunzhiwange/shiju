//! Host project ownership adapter for managed runtime selections.
use std::path::Path;
use agent_client_protocol::McpServer;
use vw_api_transport::ProjectScope;
use crate::{global_database::{GlobalDatabase, run_sync_database}, project::store::ProjectStore};
use super::McpCatalogStore;
pub use agent_app_services::mcp::runtime::*;
pub(crate) fn selected_runtime_servers(
    project: &ProjectScope,
    workspace: &Path,
    existing: &[McpServer],
) -> anyhow::Result<Vec<McpServer>> {
    let Ok(workspace) = workspace.canonicalize() else {
        return Ok(Vec::new());
    };
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    run_sync_database(|| {
        let Some(info) = ProjectStore::new(database.clone())?.get(project)? else {
            return Ok(Vec::new());
        };
        McpCatalogStore::new(database)
            .map_err(catalog_error)?
            .runtime_servers_for_project_roots(project, &workspace,
                std::iter::once(info.worktree.as_str()).chain(info.sandboxes.iter().map(String::as_str)), existing)
    })
}
