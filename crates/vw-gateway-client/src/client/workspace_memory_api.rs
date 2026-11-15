use super::{GatewayClient, WorkspaceMemoryScope, WorkspaceMemoryError, WorkspaceSearch, WorkspaceSearchResponse};
use vw_api_types::workspace_memory::*;

impl GatewayClient {
    pub async fn read_workspace_file(&self, scope: &WorkspaceMemoryScope, filename: WorkspaceFilename, token: &str) -> Result<WorkspaceFileContent, WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .read_workspace_file(scope, filename, token).await
    }
    pub async fn write_workspace_file(&self, scope: &WorkspaceMemoryScope, filename: WorkspaceFilename, content: String, token: &str) -> Result<(), WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .write_workspace_file(scope, filename, content, token).await
    }
    pub async fn read_daily_memory_file(&self, scope: &WorkspaceMemoryScope, filename: &DailyMemoryFilename, token: &str) -> Result<WorkspaceFileContent, WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .read_daily_memory_file(scope, filename, token).await
    }
    pub async fn write_daily_memory_file(&self, scope: &WorkspaceMemoryScope, filename: &DailyMemoryFilename, content: String, token: &str) -> Result<(), WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .write_daily_memory_file(scope, filename, content, token).await
    }
    pub async fn list_daily_memory_files(&self, scope: &WorkspaceMemoryScope, token: &str) -> Result<Vec<DailyMemoryFileInfo>, WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .list_daily_memory_files(scope, token).await
    }
    pub async fn delete_daily_memory_file(&self, scope: &WorkspaceMemoryScope, filename: &DailyMemoryFilename, token: &str) -> Result<(), WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .delete_daily_memory_file(scope, filename, token).await
    }
    pub async fn search_daily_memory_files(&self, scope: &WorkspaceMemoryScope, query: String, token: &str) -> Result<(WorkspaceSearch, WorkspaceSearchResponse), WorkspaceMemoryError> {
        gateway_workspace_memory::GatewayWorkspaceMemoryClient::new(self.transport.clone())
            .search_daily_memory_files(scope, query, token).await
    }
}
