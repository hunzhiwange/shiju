//! 项目档案 API 的兼容转发。
use super::GatewayClient;
pub use gateway_project_profile::project_profile_api::*;
use vw_api_types::project_profiles::*;

impl GatewayClient {
    pub async fn project_profile_list(&self, scope: &ProjectProfileApiScope) -> Result<Vec<ProjectProfile>, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_list(scope).await
    }

    pub async fn project_profile_current(&self, scope: &ProjectProfileApiScope) -> Result<ProjectProfileCurrent, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_current(scope).await
    }

    /// Snapshot availability is decided by the executor; never substitute local state.
    pub async fn project_profile_create(&self, scope: &ProjectProfileApiScope, name: &str) -> Result<ProjectProfile, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_create(scope, name).await
    }

    pub async fn project_profile_rename(&self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64, name: &str) -> Result<ProjectProfile, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_rename(scope, id, expected_version, name).await
    }

    /// Replaces only the requested application group on the execution host.
    pub async fn project_profile_resnapshot(&self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64) -> Result<ProjectProfile, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_resnapshot(scope, id, expected_version).await
    }

    pub async fn project_profile_delete(&self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64) -> Result<ProjectProfileDeleted, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_delete(scope, id, expected_version).await
    }

    /// Clears the selected marker without undoing native configuration.
    pub async fn project_profile_clear(&self, scope: &ProjectProfileApiScope) -> Result<ProjectProfileCurrent, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_clear(scope).await
    }

    /// Read-only preview on the execution host. The current server returns 503
    /// until native observations are wired; its success schema is not frozen.
    pub async fn project_profile_preview(
        &self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_preview(scope, id, expected_version).await
    }

    /// Submit exactly once using the native no-retry/no-redirect transport.
    /// On a transport error the outcome is unknown: read current/result after
    /// reconnect. Neither this method nor result reads automatically replay apply.
    /// The server currently returns 503; no successful admission schema exists yet.
    pub async fn project_profile_apply(
        &self, scope: &ProjectProfileApiScope, id: &ProjectProfileId, expected_version: u64,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_apply(scope, id, expected_version).await
    }

    /// Read the latest persisted group result, preserving partial, uncaptured,
    /// cancelled and per-item outcomes. This is not an operation-specific receipt.
    pub async fn project_profile_result(
        &self, scope: &ProjectProfileApiScope,
    ) -> Result<ProjectProfileStoredResult, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_result(scope).await
    }

    /// Query an admitted operation without restarting it. The current server has
    /// no admitted operations and returns 404; retain that structured error.
    pub async fn project_profile_progress(
        &self, scope: &ProjectProfileApiScope, operation_id: &str,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_progress(scope, operation_id).await
    }

    /// Request cancellation once. Only the executor can confirm cancellation;
    /// a failed request does not synthesize a cancelled result.
    pub async fn project_profile_cancel(
        &self, scope: &ProjectProfileApiScope, operation_id: &str,
    ) -> Result<serde_json::Value, ProjectProfileApiError> {
        gateway_project_profile::GatewayProjectProfileClient::new(self.transport.clone())
            .project_profile_cancel(scope, operation_id).await
    }

}
