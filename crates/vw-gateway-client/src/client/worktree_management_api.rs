//! Typed management endpoints; legacy path IDs must never be used here.
use super::GatewayClient;
use {
    vw_api_transport::common::OperationAck,
    vw_api_project::ProjectScopeDto,
    vw_api_worktree::*,
};
impl GatewayClient {
    pub async fn managed_worktrees(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListManagedWorktreesResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .managed_worktrees(scope).await
    }

    pub async fn managed_worktree_deletion_preflight(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<WorktreeDeletionPreflight, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .managed_worktree_deletion_preflight(scope, id).await
    }

    pub async fn managed_worktree_record_use(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<OperationAck, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .managed_worktree_record_use(scope, id).await
    }

    pub async fn managed_worktree_delete(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
        request: &DeleteManagedWorktreeRequest,
    ) -> Result<DeleteManagedWorktreeResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .managed_worktree_delete(scope, id, request).await
    }

    pub async fn worktree_snapshots(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListWorktreeSnapshotsResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .worktree_snapshots(scope).await
    }

    pub async fn worktree_snapshot_restore(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<RestoreWorktreeResponse, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .worktree_snapshot_restore(scope, id).await
    }

    pub async fn managed_worktree_retry(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<OperationAck, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .managed_worktree_retry(scope, id).await
    }

    pub async fn worktree_cleanup_proposal(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<WorktreeCleanupProposal, String> {
        gateway_project::GatewayProjectClient::new(self.transport.clone())
            .worktree_cleanup_proposal(scope).await
    }
}
