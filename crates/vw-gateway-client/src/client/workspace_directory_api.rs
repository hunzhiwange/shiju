//! The gateway opens Finder on its authorized execution host.
use super::{GatewayClient, WorkspaceMemoryError, WorkspaceMemoryScope};
use vw_api_types::workspace_memory::{WorkspaceDirectory, WorkspaceOpenDirectoryRequest};

impl GatewayClient {
    /// Create/open a fixed directory on the target executor. Dropping this future
    /// stops waiting; it cannot undo a host action already accepted by the server.
    pub async fn open_workspace_directory(
        &self, scope: &WorkspaceMemoryScope, subdir: WorkspaceDirectory, token: &str,
    ) -> Result<(), WorkspaceMemoryError> {
        let body = serde_json::to_value(WorkspaceOpenDirectoryRequest { subdir })
            .map_err(|_| WorkspaceMemoryError::InvalidInput)?;
        self.workspace_request(reqwest::Method::POST, "open-directory", scope, token, Some(body)).await?;
        Ok(())
    }
}
