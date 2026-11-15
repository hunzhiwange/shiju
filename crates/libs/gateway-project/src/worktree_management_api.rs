//! Typed management endpoints; legacy path IDs must never be used here.
use super::{GatewayProjectClient, project_api::encode_path_segment};
use {
    vw_api_transport::common::OperationAck,
    vw_api_types::project::ProjectScopeDto,
    vw_api_worktree::*,
};
impl GatewayProjectClient {
    fn management_path(&self, scope: &ProjectScopeDto, id: Option<&str>) -> Result<String, String> {
        let mut path = format!("{}/managed-worktrees", self.project_scope_path(scope)?);
        if let Some(id) = id {
            if id.trim().is_empty() {
                return Err("managed worktree id must not be empty".into());
            }
            path.push('/');
            path.push_str(&encode_path_segment(id));
        }
        Ok(path)
    }
    pub async fn managed_worktrees(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListManagedWorktreesResponse, String> {
        self.transport.get_json(&self.management_path(scope, None)?, &[]).await
    }
    pub async fn managed_worktree_deletion_preflight(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<WorktreeDeletionPreflight, String> {
        self.transport.post_json(
            &format!("{}/deletion-preflight", self.management_path(scope, Some(id))?),
            &[],
            &serde_json::json!({}),
        )
        .await
    }
    pub async fn managed_worktree_record_use(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<OperationAck, String> {
        self.transport.post_json(
            &format!("{}/record-use", self.management_path(scope, Some(id))?),
            &[],
            &serde_json::json!({}),
        )
        .await
    }
    pub async fn managed_worktree_delete(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
        request: &DeleteManagedWorktreeRequest,
    ) -> Result<DeleteManagedWorktreeResponse, String> {
        self.transport.delete_json(&self.management_path(scope, Some(id))?, &[], request).await
    }
    pub async fn worktree_snapshots(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListWorktreeSnapshotsResponse, String> {
        self.transport.get_json(&format!("{}/worktree-snapshots", self.project_scope_path(scope)?), &[]).await
    }
    pub async fn worktree_snapshot_restore(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<RestoreWorktreeResponse, String> {
        if id.trim().is_empty() {
            return Err("snapshot id must not be empty".into());
        }
        self.transport.post_json(
            &format!(
                "{}/worktree-snapshots/{}/restore",
                self.project_scope_path(scope)?,
                encode_path_segment(id)
            ),
            &[],
            &serde_json::json!({}),
        )
        .await
    }
    pub async fn managed_worktree_retry(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<OperationAck, String> {
        let path = format!("{}/retry-initialization", self.management_path(scope, Some(id))?);
        let builder = self.transport.client()
            .post(format!("{}{}", self.transport.endpoint().base_url(), path))
            .json(&serde_json::json!({}));
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(std::time::Duration::from_secs(270));
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
            .send()
            .await
            .map_err(|error| gateway_http::transport_error("POST", self.transport.endpoint(), &path, error))?;
        gateway_http::parse_json_response("POST", self.transport.endpoint(), &path, response).await
    }
    pub async fn worktree_cleanup_proposal(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<WorktreeCleanupProposal, String> {
        self.transport.get_json(
            &format!("{}/worktree-cleanup-proposal", self.project_scope_path(scope)?),
            &[],
        )
        .await
    }
}
#[cfg(test)]
#[path = "worktree_management_api_tests.rs"]
mod tests;
