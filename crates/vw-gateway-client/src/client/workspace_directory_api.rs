//! The gateway opens Finder on its authorized execution host.
use super::{GatewayClient, WorkspaceMemoryError, WorkspaceMemoryScope};
use vw_api_workspace_memory::{WorkspaceDirectory};

impl GatewayClient {
    /// Create/open a fixed directory on the target executor. Dropping this future
    /// stops waiting; it cannot undo a host action already accepted by the server.
    pub async fn open_workspace_directory(
        &self, scope: &WorkspaceMemoryScope, subdir: WorkspaceDirectory, token: &str,
    ) -> Result<(), WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .open_workspace_directory(scope, subdir, token).await
    }
}
