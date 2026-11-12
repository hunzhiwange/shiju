//! Typed management endpoints; legacy path IDs must never be used here.
use super::{GatewayClient, project_api::encode_path_segment};
use vw_api_types::{common::OperationAck, project::ProjectScopeDto, worktree::*};
impl GatewayClient {
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
        self.get_json(&self.management_path(scope, None)?, &[]).await
    }
    pub async fn managed_worktree_deletion_preflight(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<WorktreeDeletionPreflight, String> {
        self.post_json(
            &format!("{}/deletion-preflight", self.management_path(scope, Some(id))?),
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
        self.delete_json(&self.management_path(scope, Some(id))?, &[], request).await
    }
    pub async fn worktree_snapshots(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<ListWorktreeSnapshotsResponse, String> {
        self.get_json(&format!("{}/worktree-snapshots", self.project_scope_path(scope)?), &[]).await
    }
    pub async fn worktree_snapshot_restore(
        &self,
        scope: &ProjectScopeDto,
        id: &str,
    ) -> Result<RestoreWorktreeResponse, String> {
        if id.trim().is_empty() {
            return Err("snapshot id must not be empty".into());
        }
        self.post_json(
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
        let builder = self
            .client
            .post(format!("{}{}", self.endpoint.base_url(), path))
            .json(&serde_json::json!({}));
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(std::time::Duration::from_secs(270));
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|error| crate::http::transport_error("POST", &self.endpoint, &path, error))?;
        crate::http::parse_json_response("POST", &self.endpoint, &path, response).await
    }
    pub async fn worktree_cleanup_proposal(
        &self,
        scope: &ProjectScopeDto,
    ) -> Result<WorktreeCleanupProposal, String> {
        self.get_json(
            &format!("{}/worktree-cleanup-proposal", self.project_scope_path(scope)?),
            &[],
        )
        .await
    }
}
#[cfg(test)]
#[path = "worktree_management_api_tests.rs"]
mod tests;
